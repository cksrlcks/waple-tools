//! 작업 기록. 한 줄 = 한 동작(JSONL). 이동할 때마다 바로 쓰므로 중간에 꺼져도 그때까지는 되돌릴 수 있다.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum LogEntry {
    /// 원본을 백업으로 옮김. `hash`는 옮긴 뒤 `to`의 해시
    Moved {
        from: PathBuf,
        to: PathBuf,
        hash: String,
    },
    /// 결과 파일을 만듦
    Created { path: PathBuf, hash: String },
}

pub struct JobLog {
    file: Mutex<File>,
}

pub(crate) fn log_path(log_dir: &Path, job_id: &str) -> PathBuf {
    log_dir.join(format!("{job_id}.jsonl"))
}

impl JobLog {
    /// 새 기록 파일을 만든다. 같은 job_id가 이미 있으면 오류.
    pub fn create(log_dir: &Path, job_id: &str) -> io::Result<Self> {
        std::fs::create_dir_all(log_dir)?;
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(log_path(log_dir, job_id))?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub fn append(&self, e: &LogEntry) -> io::Result<()> {
        let mut line = serde_json::to_string(e).map_err(io::Error::other)?;
        line.push('\n');
        let mut f = self.file.lock().unwrap_or_else(|e| e.into_inner());
        f.write_all(line.as_bytes())?;
        f.flush()
    }
}

/// 기록을 순서대로 읽는다. 앱이 꺼지며 반쯤 쓴 마지막 줄은 버린다.
pub fn read_log(log_dir: &Path, job_id: &str) -> io::Result<Vec<LogEntry>> {
    let text = std::fs::read_to_string(log_path(log_dir, job_id))?;
    Ok(text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}
