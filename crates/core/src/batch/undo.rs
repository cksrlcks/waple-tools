//! 작업 기록을 역순으로 적용해 되돌린다. 그사이 사용자가 고친 파일은 건드리지 않는다.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::collect::BACKUP_DIR;
use super::fsops::{file_hash, move_new};
use super::log::{LogEntry, log_path, read_log};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UndoItem {
    /// 원본을 원래 자리로 되돌림
    Restored {
        path: PathBuf,
    },
    /// 결과 파일을 지움
    Removed {
        path: PathBuf,
    },
    Skipped {
        path: PathBuf,
        reason: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum UndoError {
    #[error("작업 기록을 찾을 수 없습니다.")]
    NotFound,
    #[error("이미 되돌린 작업입니다.")]
    AlreadyUndone,
    #[error("파일 오류: {0}")]
    Io(#[from] io::Error),
}

fn undone_path(log_dir: &Path, job_id: &str) -> PathBuf {
    log_dir.join(format!("{job_id}.undone.jsonl"))
}

pub fn undo(log_dir: &Path, job_id: &str) -> Result<Vec<UndoItem>, UndoError> {
    if undone_path(log_dir, job_id).exists() {
        return Err(UndoError::AlreadyUndone);
    }
    if !log_path(log_dir, job_id).exists() {
        return Err(UndoError::NotFound);
    }
    let entries = read_log(log_dir, job_id)?;
    let items: Vec<UndoItem> = entries.iter().rev().map(undo_one).collect();
    // 파일 잠금처럼 일시적인 이유로 건너뛴 항목이 있으면 다시 시도할 수 있게 기록을 남겨 둔다.
    // 다시 실행하면 이미 처리된 항목은 "이미 없음"/"백업이 없음"으로 건너뛰어지고 나머지만 처리된다.
    if items.iter().all(|i| !transient(i)) {
        std::fs::rename(log_path(log_dir, job_id), undone_path(log_dir, job_id))?;
    }
    Ok(items)
}

/// 다시 시도하면 처리될 수 있는 건너뜀인가
fn transient(i: &UndoItem) -> bool {
    matches!(i, UndoItem::Skipped { reason, .. }
        if !matches!(reason.as_str(), "이미 없음" | "수정됨" | "백업이 없음"))
}

fn undo_one(e: &LogEntry) -> UndoItem {
    let skipped = |path: &Path, reason: &str| UndoItem::Skipped {
        path: path.to_path_buf(),
        reason: reason.into(),
    };
    match e {
        LogEntry::Created { path, hash } => {
            if !path.exists() {
                return skipped(path, "이미 없음");
            }
            if !same_hash(path, hash) {
                return skipped(path, "수정됨");
            }
            match std::fs::remove_file(path) {
                Ok(()) => UndoItem::Removed { path: path.clone() },
                Err(err) => skipped(path, &err.to_string()),
            }
        }
        LogEntry::Moved { from, to, hash } => {
            if !to.exists() {
                return skipped(from, "백업이 없음");
            }
            if !same_hash(to, hash) {
                return skipped(from, "수정됨");
            }
            if from.exists() {
                return skipped(from, "원래 자리에 다른 파일이 있음");
            }
            match move_new(to, from) {
                Ok(_) => {
                    remove_empty_backup_dirs(to);
                    UndoItem::Restored { path: from.clone() }
                }
                Err(err) => skipped(from, &err.to_string()),
            }
        }
    }
}

fn same_hash(p: &Path, hash: &str) -> bool {
    file_hash(p).is_ok_and(|h| h == hash)
}

/// 백업 파일이 있던 폴더부터 `original` 폴더까지, 비어 있는 것만 지운다.
fn remove_empty_backup_dirs(backup_file: &Path) {
    let mut dir = backup_file.parent();
    while let Some(d) = dir {
        if std::fs::remove_dir(d).is_err() || d.file_name().is_some_and(|n| n == BACKUP_DIR) {
            break;
        }
        dir = d.parent();
    }
}
