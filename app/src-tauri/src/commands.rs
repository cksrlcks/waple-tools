use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Channel;
use waple_tools_builtin::{
    Settings, builtin, catalog, default_log_dir, load_settings, save_settings, settings_path,
};
use waple_tools_core::{Block, Tool, ToolKind};

use crate::addons::{AddonStatus, Addons, DownloadProgress};
use waple_tools_core::batch::{self, JobReport, JobSpec, Plan, Progress, UndoItem};

const NOT_FOUND: &str = "도구를 찾을 수 없습니다.";
const RUN_FAILED: &str = "작업 실행 중 오류가 발생했습니다.";

/// 실행 중인 작업별 취소 플래그
#[derive(Default)]
pub struct Jobs(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl Jobs {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 작업 중 오프라인: 실행 중인 작업이 있으면 다운로드를 시작하지 않는다
    fn ensure_idle(&self) -> Result<(), String> {
        if self.lock().is_empty() {
            Ok(())
        } else {
            Err("작업이 끝난 뒤 설치할 수 있습니다.".into())
        }
    }
}

/// 기본 내장 → 설치된 추가 도구 순으로 찾는다
fn find(addons: &Addons, id: &str) -> Option<Arc<dyn Tool>> {
    waple_tools_builtin::find(id)
        .map(Arc::from)
        .or_else(|| addons.get(id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolStatus {
    Builtin,
    Installed,
    NotInstalled,
    UpdateAvailable,
    UpdateRequired,
}

impl From<AddonStatus> for ToolStatus {
    fn from(s: AddonStatus) -> Self {
        match s {
            AddonStatus::Installed => Self::Installed,
            AddonStatus::NotInstalled => Self::NotInstalled,
            AddonStatus::UpdateAvailable => Self::UpdateAvailable,
            AddonStatus::UpdateRequired => Self::UpdateRequired,
        }
    }
}

/// 카탈로그 항목 + 설치 상태. 쓸 수 있는 도구만 kind·스키마·확장자가 있다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolEntry {
    id: String,
    name: String,
    category: String,
    description: String,
    status: ToolStatus,
    kind: Option<ToolKind>,
    options_schema: Option<Value>,
    extensions: Vec<String>,
    /// 추가 도구: 설치된 버전, 이 앱용 최신 버전, 다운로드 크기
    version: Option<String>,
    latest: Option<String>,
    size: Option<u64>,
}

#[tauri::command]
pub fn list_tools(addons: tauri::State<'_, Addons>) -> Vec<ToolEntry> {
    let tools: Vec<Arc<dyn Tool>> = builtin().into_iter().map(Arc::from).collect();
    catalog()
        .into_iter()
        .map(|e| {
            let builtin = tools.iter().find(|t| t.meta().id == e.id).cloned();
            let info = (!e.builtin).then(|| addons.info(&e.id));
            let tool = builtin.clone().or_else(|| addons.get(&e.id));
            let tool = tool.as_ref();
            ToolEntry {
                status: match (&builtin, &info) {
                    (Some(_), _) => ToolStatus::Builtin,
                    (None, Some(i)) => i.status.into(),
                    (None, None) => ToolStatus::NotInstalled,
                },
                version: info.as_ref().and_then(|i| i.version.clone()),
                latest: info.as_ref().and_then(|i| i.latest.clone()),
                size: info.as_ref().and_then(|i| i.size),
                kind: tool.map(|t| t.kind()),
                options_schema: tool.map(|t| t.options_schema()),
                extensions: tool.map(|t| t.meta().extensions).unwrap_or_default(),
                id: e.id,
                name: e.name,
                category: e.category,
                description: e.description,
            }
        })
        .collect()
}

/// 실행 전에 받아 두어야 실행 중 취소 버튼이 job_id를 안다.
#[tauri::command]
pub fn new_job_id() -> String {
    waple_tools_builtin::new_job_id()
}

/// 추가 도구 설치·업데이트·다시 설치
#[tauri::command]
pub async fn install_tool(
    addons: tauri::State<'_, Addons>,
    jobs: tauri::State<'_, Jobs>,
    tool_id: String,
    on_progress: Channel<DownloadProgress>,
) -> Result<(), String> {
    if !catalog().iter().any(|e| e.id == tool_id && !e.builtin) {
        return Err(NOT_FOUND.into());
    }
    jobs.ensure_idle()?;
    addons
        .install(&tool_id, |p| {
            let _ = on_progress.send(p);
        })
        .await
}

#[tauri::command]
pub fn uninstall_tool(addons: tauri::State<'_, Addons>, tool_id: String) -> Result<(), String> {
    addons.uninstall(&tool_id)
}

/// 원격 목록을 다시 받는다 (앱 시작 시 한 번, 업데이트 표시용)
#[tauri::command]
pub async fn refresh_addons(
    addons: tauri::State<'_, Addons>,
    jobs: tauri::State<'_, Jobs>,
) -> Result<(), String> {
    jobs.ensure_idle()?;
    addons.refresh().await
}

#[tauri::command]
pub async fn preview_job(
    addons: tauri::State<'_, Addons>,
    tool_id: String,
    spec: JobSpec,
) -> Result<Plan, String> {
    let tool = find(&addons, &tool_id).ok_or(NOT_FOUND)?;
    tauri::async_runtime::spawn_blocking(move || batch::plan(&*tool, &spec))
        .await
        .map_err(|_| RUN_FAILED.to_string())
}

#[tauri::command]
pub async fn run_job(
    addons: tauri::State<'_, Addons>,
    jobs: tauri::State<'_, Jobs>,
    job_id: String,
    tool_id: String,
    spec: JobSpec,
    on_progress: Channel<Progress>,
) -> Result<JobReport, String> {
    let tool = find(&addons, &tool_id).ok_or(NOT_FOUND)?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut map = jobs.lock();
        if map.contains_key(&job_id) {
            return Err("이미 실행 중인 작업입니다.".into());
        }
        map.insert(job_id.clone(), cancel.clone());
    }
    let id = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        batch::run(&*tool, &spec, &id, &default_log_dir(), cancel, move |p| {
            let _ = on_progress.send(p);
        })
    })
    .await;
    jobs.lock().remove(&job_id);
    result
        .map_err(|_| RUN_FAILED.to_string())?
        .map_err(|_| "작업 기록을 만들 수 없습니다.".to_string())
}

#[tauri::command]
pub fn cancel_job(jobs: tauri::State<'_, Jobs>, job_id: String) {
    if let Some(flag) = jobs.lock().get(&job_id) {
        flag.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn undo_job(job_id: String) -> Result<Vec<UndoItem>, String> {
    tauri::async_runtime::spawn_blocking(move || batch::undo(&default_log_dir(), &job_id))
        .await
        .map_err(|_| RUN_FAILED.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings() -> Settings {
    load_settings(&settings_path())
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    save_settings(&settings_path(), &settings).map_err(|_| "설정을 저장할 수 없습니다.".to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathStat {
    path: PathBuf,
    /// 폴더는 0
    size: u64,
    is_dir: bool,
    exists: bool,
}

#[tauri::command]
pub fn stat_paths(paths: Vec<PathBuf>) -> Vec<PathStat> {
    paths
        .into_iter()
        .map(|path| {
            let meta = std::fs::metadata(&path).ok();
            PathStat {
                size: meta.as_ref().filter(|m| m.is_file()).map_or(0, |m| m.len()),
                is_dir: meta.as_ref().is_some_and(|m| m.is_dir()),
                exists: meta.is_some(),
                path,
            }
        })
        .collect()
}

/// 화면용 이미지 (data URL)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    url: String,
    /// 원본 크기
    width: u32,
    height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPreview {
    before: Shown,
    /// 결과가 이미지가 아니면 없음
    after: Option<Shown>,
    size_before: u64,
    size_after: Option<u64>,
    /// 도구가 실패한 이유 (원본은 그대로 보여 준다)
    error: Option<String>,
}

fn shown(path: &std::path::Path, max_side: u32) -> Result<Shown, String> {
    use base64::Engine as _;
    let d = waple_tools_image_common::display_image(path, max_side).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&d.jpeg);
    Ok(Shown {
        url: format!("data:image/jpeg;base64,{b64}"),
        width: d.width,
        height: d.height,
    })
}

/// 입력 목록 썸네일 (긴 변 160px)
#[tauri::command]
pub async fn thumbnail(path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _slot = crate::view_limit::VIEW.acquire();
        shown(&path, 160).map(|s| s.url)
    })
    .await
    .map_err(|_| RUN_FAILED.to_string())?
}

/// 원본 ↔ 결과 비교: 파일 하나에 도구를 임시로 실행한다 (원본·작업 기록 그대로)
#[tauri::command]
pub async fn preview_tool(
    addons: tauri::State<'_, Addons>,
    tool_id: String,
    input: PathBuf,
    opts: Value,
) -> Result<ToolPreview, String> {
    let tool = find(&addons, &tool_id).ok_or(NOT_FOUND)?;
    let id = crate::view_limit::next_preview();
    tauri::async_runtime::spawn_blocking(move || {
        use crate::view_limit::{PREVIEW_LOCK, VIEW, is_latest_preview};
        // 한 번에 하나만, 기다리는 사이 더 새 요청이 왔으면 건너뛴다 (화면은 이미 새 요청을 기다림)
        let _one = PREVIEW_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if !is_latest_preview(id) {
            return Err("새 옵션으로 다시 만드는 중입니다.".to_string());
        }
        let _slot = VIEW.acquire();
        const MAX: u32 = 1600;
        let size_before = std::fs::metadata(&input)
            .map(|m| m.len())
            .map_err(|e| e.to_string())?;
        let before = shown(&input, MAX)?;
        // 도구가 실패해도(예: 아무 옵션도 안 고름) 원본은 보여 준다
        let (after, size_after, error) = match batch::preview_one(&*tool, &input, &opts) {
            Ok((_tmp, files)) => {
                let first = files.first();
                (
                    first.and_then(|f| shown(f, MAX).ok()),
                    first
                        .and_then(|f| std::fs::metadata(f).ok())
                        .map(|m| m.len()),
                    None,
                )
            }
            Err(e) => (None, None, Some(e)),
        };
        Ok(ToolPreview {
            before,
            after,
            size_before,
            size_after,
            error,
        })
    })
    .await
    .map_err(|_| RUN_FAILED.to_string())?
}

/// Interactive 도구: 파일 하나에 실행해 결과 블록을 돌려준다 (파일을 만들지 않는다)
#[tauri::command]
pub async fn run_interactive(
    addons: tauri::State<'_, Addons>,
    tool_id: String,
    input: PathBuf,
    opts: Value,
) -> Result<Vec<Block>, String> {
    let tool = find(&addons, &tool_id).ok_or(NOT_FOUND)?;
    tauri::async_runtime::spawn_blocking(move || batch::interactive(&*tool, &input, &opts))
        .await
        .map_err(|_| RUN_FAILED.to_string())?
}

/// 파일 › 종료: 트레이 보관 설정과 상관없이 앱을 끝낸다
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
