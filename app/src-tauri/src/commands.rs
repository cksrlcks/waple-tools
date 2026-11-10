use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use localtools_core::{ToolKind, ToolMeta};
use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Channel;

use crate::builtin::builtin;
use crate::job::{FileResult, Progress, run_files};

// ponytail: 작업 하나만 취소 가능, 동시 작업은 Phase 1 배치 엔진에서 job_id별로
#[derive(Default)]
pub struct CancelFlag(pub Arc<AtomicBool>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    meta: ToolMeta,
    kind: ToolKind,
    options_schema: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    job_id: String,
    files: Vec<FileResult>,
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

#[tauri::command]
pub async fn run_job(
    cancel: tauri::State<'_, CancelFlag>,
    tool_id: String,
    inputs: Vec<PathBuf>,
    opts: Value,
    on_progress: Channel<Progress>,
) -> Result<RunResult, String> {
    let tool = builtin()
        .into_iter()
        .find(|t| t.meta().id == tool_id)
        .ok_or("도구를 찾을 수 없습니다.")?;
    let flag = cancel.0.clone();
    flag.store(false, Ordering::Relaxed);
    let job_id = new_job_id();
    let job_dir = std::env::temp_dir().join("localtools").join(&job_id);
    let files = tauri::async_runtime::spawn_blocking(move || {
        run_files(&*tool, &inputs, &job_dir, &opts, flag, move |p| {
            let _ = on_progress.send(p);
        })
    })
    .await
    .map_err(|_| "작업 실행 중 오류가 발생했습니다.".to_string())?;
    Ok(RunResult { job_id, files })
}

#[tauri::command]
pub fn cancel_job(cancel: tauri::State<'_, CancelFlag>) {
    cancel.0.store(true, Ordering::Relaxed);
}

/// 밀리초 타임스탬프. 같은 밀리초에 두 작업이 시작될 일은 없다(작업은 하나씩).
fn new_job_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    ms.to_string()
}
