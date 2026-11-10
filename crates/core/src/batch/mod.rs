//! 배치 엔진: 입력 모으기(dry-run) → 파일별 실행 → 백업·결과 이동 → 로그 → 되돌리기.
//! 도구는 임시 폴더에만 쓰고, 원본을 옮기는 것은 이 모듈뿐이다.

mod collect;
pub mod fsops;
mod log;
mod prune;
mod run;
mod undo;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use collect::plan;
pub use log::{JobLog, LogEntry, read_log};
pub use prune::prune_logs;
pub use run::{effective_threads, run};
pub use undo::{UndoError, UndoItem, undo};

/// 파일 하나의 처리 결과
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FileResult {
    Done {
        input: PathBuf,
        /// 최종 결과 경로
        outputs: Vec<PathBuf>,
        /// InPlaceWithBackup일 때 원본이 옮겨진 곳
        backup: Option<PathBuf>,
        size_before: u64,
        size_after: u64,
    },
    Skipped {
        input: PathBuf,
        reason: String,
    },
    Failed {
        input: PathBuf,
        message: String,
    },
    Cancelled {
        input: PathBuf,
    },
}

/// 끝난 파일 수 (병렬이라 파일 안의 비율은 보내지 않는다)
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobReport {
    pub job_id: String,
    pub files: Vec<FileResult>,
    /// 계획 단계에서 제외된 것 (지원하지 않는 형식 등)
    pub skipped: Vec<Skipped>,
}

/// 결과를 어디에 둘지 (plan.md 3.4)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputPolicy {
    /// 결과를 원본 자리에, 원본은 `<작업 루트>/original/`로
    #[default]
    InPlaceWithBackup,
    /// 원본 그대로, 결과는 `이름{suffix}.ext`
    KeepOriginal { suffix: String },
    /// 원본 그대로, 결과는 `dir` 아래 (하위 폴더 구조 유지)
    OutputFolder { dir: PathBuf },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSpec {
    pub inputs: Vec<PathBuf>,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub policy: OutputPolicy,
    /// 결과가 원본보다 크면 원본 유지 (기본 켜짐)
    #[serde(default = "yes")]
    pub keep_if_larger: bool,
    /// 0이면 CPU 코어 수 - 1
    #[serde(default)]
    pub threads: usize,
    #[serde(default = "empty_object")]
    pub opts: serde_json::Value,
}

fn yes() -> bool {
    true
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub input: PathBuf,
    /// 폴더 입력이면 그 폴더, 파일 입력이면 파일의 부모 폴더
    pub root: PathBuf,
    /// root 기준 상대 경로
    pub rel: PathBuf,
    /// 결과가 놓일 폴더
    pub dest_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: String,
}

/// dry-run 결과
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub files: Vec<PlannedFile>,
    pub skipped: Vec<Skipped>,
}
