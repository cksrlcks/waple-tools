use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use localtools_builtin::{builtin, default_log_dir, find};
use localtools_core::batch::{self, JobReport, JobSpec, Plan, Progress, UndoItem};
use localtools_core::{ToolKind, ToolMeta};
use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Channel;

const NOT_FOUND: &str = "도구를 찾을 수 없습니다.";
const RUN_FAILED: &str = "작업 실행 중 오류가 발생했습니다.";

/// 실행 중인 작업별 취소 플래그
#[derive(Default)]
pub struct Jobs(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl Jobs {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    meta: ToolMeta,
    kind: ToolKind,
    options_schema: Value,
}

#[tauri::command]
pub fn list_tools() -> Vec<ToolInfo> {
    builtin()
        .into_iter()
        .map(|t| ToolInfo {
            meta: t.meta(),
            kind: t.kind(),
            options_schema: t.options_schema(),
        })
        .collect()
}

/// 실행 전에 받아 두어야 실행 중 취소 버튼이 job_id를 안다.
#[tauri::command]
pub fn new_job_id() -> String {
    localtools_builtin::new_job_id()
}

#[tauri::command]
pub async fn preview_job(tool_id: String, spec: JobSpec) -> Result<Plan, String> {
    let tool = find(&tool_id).ok_or(NOT_FOUND)?;
    tauri::async_runtime::spawn_blocking(move || batch::plan(&*tool, &spec))
        .await
        .map_err(|_| RUN_FAILED.to_string())
}

#[tauri::command]
pub async fn run_job(
    jobs: tauri::State<'_, Jobs>,
    job_id: String,
    tool_id: String,
    spec: JobSpec,
    on_progress: Channel<Progress>,
) -> Result<JobReport, String> {
    let tool = find(&tool_id).ok_or(NOT_FOUND)?;
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
