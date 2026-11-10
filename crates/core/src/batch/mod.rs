//! 배치 엔진: 입력 모으기(dry-run) → 파일별 실행 → 백업·결과 이동 → 로그 → 되돌리기.
//! 도구는 임시 폴더에만 쓰고, 원본을 옮기는 것은 이 모듈뿐이다.

mod collect;
pub mod fsops;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use collect::plan;

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
