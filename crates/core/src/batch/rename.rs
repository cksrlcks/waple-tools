//! 파일명 일괄 변경 (`ToolKind::Rename`): 도구는 새 이름만 정하고, 충돌 확인·이름 바꾸기·기록은 여기서 한다.
//! 원본을 지우거나 덮어쓰지 않는다. `original/` 백업 없이 기록(`Moved`)만으로 되돌린다.

use std::collections::{HashMap, HashSet};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use super::collect::{TMP_PREFIX, plan as collect};
use super::fsops::{file_hash, move_exact_logged, move_new_logged};
use super::log::{JobLog, LogEntry};
use super::run::PANIC_MESSAGE;
use super::{FileResult, JobReport, JobSpec, Progress, Skipped};
use crate::{JobContext, Tool, ToolError, ToolKind, ToolOutput};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RenameStatus {
    /// 이름이 바뀐다
    Changed,
    /// 새 이름이 지금 이름과 같다
    Unchanged,
    /// 바꿀 수 없다. `reason`은 화면에 그대로 보인다
    Conflict { reason: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRow {
    pub from: PathBuf,
    /// 새 경로 (같은 폴더)
    pub to: PathBuf,
    /// 도구가 정한 새 이름 그대로 (쓸 수 없는 글자가 있어도 보여 주려고 따로 둔다)
    pub name: String,
    #[serde(flatten)]
    pub status: RenameStatus,
}

/// 변경 전/후 미리보기 (dry-run). 파일 시스템은 바꾸지 않는다
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlan {
    pub rows: Vec<RenameRow>,
    pub skipped: Vec<Skipped>,
}

/// 미리보기 표. 도구 오류(예: 잘못된 정규식)는 사용자에게 보일 문구로
pub fn plan(tool: &dyn Tool, spec: &JobSpec) -> Result<RenamePlan, String> {
    let (files, skipped) = inputs(tool, spec);
    let names = new_names(tool, &files, &spec.opts)?;
    Ok(RenamePlan {
        rows: check(&files, &names),
        skipped,
    })
}

/// 실행: 지금 상태로 다시 확인한 뒤 바뀜 줄만 이름을 바꾸고 기록한다. 그사이 생긴 충돌은 그 줄만 실패.
/// 서로 이름을 맞바꾸거나 대소문자만 바꾸는 줄은 같은 폴더의 임시 이름을 거친다 (두 단계 이동, 모두 기록).
/// `io::Error`는 기록 파일을 만들 수 없을 때만.
pub fn apply(
    tool: &dyn Tool,
    spec: &JobSpec,
    job_id: &str,
    log_dir: &Path,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(Progress),
) -> io::Result<JobReport> {
    let (files, skipped) = inputs(tool, spec);
    let log = JobLog::create(log_dir, job_id)?;
    let report = |files: Vec<FileResult>| JobReport {
        job_id: job_id.to_string(),
        files,
        skipped: skipped.clone(),
    };
    let rows = match new_names(tool, &files, &spec.opts) {
        Ok(names) => check(&files, &names),
        Err(message) => {
            let failed = files
                .into_iter()
                .map(|input| FileResult::Failed {
                    input,
                    message: message.clone(),
                })
                .collect();
            return Ok(report(failed));
        }
    };

    let total = rows.len();
    let send = |done: usize| {
        on_progress(Progress {
            done,
            total,
            ratio: done as f32 / total.max(1) as f32,
        })
    };
    let leaving: HashSet<String> = rows
        .iter()
        .filter(|r| r.status == RenameStatus::Changed)
        .map(|r| key(&r.from))
        .collect();
    let mut results: Vec<Option<FileResult>> = vec![None; total];
    let mut done = 0;
    let mut finish = |results: &mut Vec<Option<FileResult>>, i: usize, r: FileResult| {
        results[i] = Some(r);
        done += 1;
        send(done);
    };
    let failed = |r: &RenameRow, message: String| FileResult::Failed {
        input: r.from.clone(),
        message,
    };

    // 1단계: 다른 줄이 비울 이름(또는 대소문자만 다른 자기 이름)으로 갈 줄은 임시 이름으로 먼저 비킨다
    let mut direct = Vec::new();
    let mut staged = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match &r.status {
            RenameStatus::Unchanged => finish(
                &mut results,
                i,
                FileResult::Skipped {
                    input: r.from.clone(),
                    reason: "이름 그대로".into(),
                },
            ),
            RenameStatus::Conflict { reason } => finish(&mut results, i, failed(r, reason.clone())),
            RenameStatus::Changed if cancel.load(Ordering::Relaxed) => finish(
                &mut results,
                i,
                FileResult::Cancelled {
                    input: r.from.clone(),
                },
            ),
            RenameStatus::Changed => {
                let hash = match file_hash(&r.from) {
                    Ok(h) => h,
                    Err(e) => {
                        finish(&mut results, i, failed(r, io_message(e)));
                        continue;
                    }
                };
                if !leaving.contains(&key(&r.to)) {
                    direct.push((i, hash));
                    continue;
                }
                let want = r.from.with_file_name(format!("{TMP_PREFIX}{job_id}-{i}"));
                let moved = move_new_logged(&r.from, &want, |t| {
                    log.append(&LogEntry::Moved {
                        from: r.from.clone(),
                        to: t.to_path_buf(),
                        hash: hash.clone(),
                    })
                });
                match moved {
                    Ok(tmp) => staged.push((i, tmp, hash)),
                    Err(e) => finish(&mut results, i, failed(r, io_message(e))),
                }
            }
        }
    }

    // 2단계: 아무도 쓰지 않는 이름으로 가는 줄
    for (i, hash) in direct {
        let r = &rows[i];
        let moved = move_exact_logged(&r.from, &r.to, |t| {
            log.append(&LogEntry::Moved {
                from: r.from.clone(),
                to: t.to_path_buf(),
                hash,
            })
        });
        let result = match moved {
            Ok(()) => done_result(r),
            Err(e) => failed(r, io_message(e)),
        };
        finish(&mut results, i, result);
    }

    // 3단계: 임시 이름 → 새 이름. 안 되면 원래 이름으로 (그 이름도 생겼으면 번호 붙인 이름) — 임시 이름으로 남기지 않는다
    for (i, tmp, hash) in staged {
        let r = &rows[i];
        let record = |t: &Path| {
            log.append(&LogEntry::Moved {
                from: tmp.clone(),
                to: t.to_path_buf(),
                hash: hash.clone(),
            })
        };
        let result = match move_exact_logged(&tmp, &r.to, record) {
            Ok(()) => done_result(r),
            Err(e) => {
                let message = io_message(e);
                match move_new_logged(&tmp, &r.from, record) {
                    Ok(_) => failed(r, message),
                    Err(_) => failed(r, format!("{message} (임시 이름 {}로 남음)", tmp.display())),
                }
            }
        };
        finish(&mut results, i, result);
    }
    Ok(report(results.into_iter().flatten().collect()))
}

fn done_result(r: &RenameRow) -> FileResult {
    let size = std::fs::metadata(&r.to).map(|m| m.len()).unwrap_or(0);
    FileResult::Done {
        input: r.from.clone(),
        outputs: vec![r.to.clone()],
        backup: None,
        size_before: size,
        size_after: size,
    }
}

fn io_message(e: io::Error) -> String {
    if e.kind() == io::ErrorKind::AlreadyExists {
        "같은 이름의 파일이 생겨 바꾸지 않았습니다.".into()
    } else {
        ToolError::from(e).to_string()
    }
}

/// 입력(파일·폴더)을 파일 목록으로. 폴더는 다른 작업과 같은 규칙(`recursive`, 제외 폴더)으로 펼친다
fn inputs(tool: &dyn Tool, spec: &JobSpec) -> (Vec<PathBuf>, Vec<Skipped>) {
    let p = collect(tool, spec);
    (p.files.into_iter().map(|f| f.input).collect(), p.skipped)
}

/// 도구에 입력 순서대로 새 이름을 묻는다 (파일을 건드리지 않는 계산)
fn new_names(
    tool: &dyn Tool,
    files: &[PathBuf],
    opts: &serde_json::Value,
) -> Result<Vec<String>, String> {
    if tool.kind() != ToolKind::Rename {
        return Err("지원하지 않는 도구 종류입니다.".into());
    }
    if files.is_empty() {
        return Ok(vec![]);
    }
    let tmp = tempfile::Builder::new()
        .prefix("waple-tools-rename-")
        .tempdir()
        .map_err(|e| ToolError::from(e).to_string())?;
    let ctx = JobContext::new(|_| {}, Default::default());
    let names = match catch_unwind(AssertUnwindSafe(|| tool.run(&ctx, files, tmp.path(), opts))) {
        Err(_) => return Err(PANIC_MESSAGE.into()),
        Ok(Err(e)) => return Err(e.to_string()),
        Ok(Ok(ToolOutput::Blocks(_))) => return Err("지원하지 않는 도구 종류입니다.".into()),
        Ok(Ok(ToolOutput::Files(names))) => names,
    };
    if names.len() != files.len() {
        return Err("도구가 돌려준 이름 수가 파일 수와 다릅니다.".into());
    }
    Ok(names
        .into_iter()
        .map(|n| n.to_string_lossy().into_owned())
        .collect())
}

/// 같은 이름인지 비교할 열쇠 (Windows는 대소문자 구분 없음)
fn key(p: &Path) -> String {
    let s = p.to_string_lossy();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s.into_owned()
    }
}

/// 상태 정하기: 이름 규칙 → 새 이름끼리 겹침 → 디스크에 이미 있음 (비켜 줄 파일이 바뀔 수 없게 되면 다시)
fn check(files: &[PathBuf], names: &[String]) -> Vec<RenameRow> {
    let conflict = |reason: &str| RenameStatus::Conflict {
        reason: reason.into(),
    };
    let mut rows: Vec<RenameRow> = files
        .iter()
        .zip(names)
        .map(|(from, name)| {
            let status = if from.file_name().is_some_and(|n| n == name.as_str()) {
                RenameStatus::Unchanged
            } else if let Some(reason) = invalid_name(name) {
                conflict(reason)
            } else {
                RenameStatus::Changed
            };
            RenameRow {
                from: from.clone(),
                to: from.with_file_name(name),
                name: name.clone(),
                status,
            }
        })
        .collect();

    let mut count: HashMap<String, usize> = HashMap::new();
    for r in rows.iter().filter(|r| r.status == RenameStatus::Changed) {
        *count.entry(key(&r.to)).or_default() += 1;
    }
    for r in rows
        .iter_mut()
        .filter(|r| r.status == RenameStatus::Changed)
    {
        if count[&key(&r.to)] > 1 {
            r.status = conflict("다른 파일의 새 이름과 같음");
        }
    }

    // 이미 있는 이름은 그 파일도 이번에 다른 이름으로 바뀔 때만 쓸 수 있다
    loop {
        let leaving: HashSet<String> = rows
            .iter()
            .filter(|r| r.status == RenameStatus::Changed)
            .map(|r| key(&r.from))
            .collect();
        let mut again = false;
        for r in rows
            .iter_mut()
            .filter(|r| r.status == RenameStatus::Changed)
        {
            if std::fs::symlink_metadata(&r.to).is_ok() && !leaving.contains(&key(&r.to)) {
                r.status = conflict("같은 이름의 파일이 이미 있음");
                again = true;
            }
        }
        if !again {
            return rows;
        }
    }
}

/// Windows에서 쓸 수 없는 파일 이름이면 이유
fn invalid_name(name: &str) -> Option<&'static str> {
    if name.is_empty() || name == "." || name == ".." {
        return Some("새 이름이 비어 있음");
    }
    if name.chars().any(|c| c.is_control()) {
        return Some("제어 문자가 있음");
    }
    if name.chars().any(|c| r#"<>:"/\|?*"#.contains(c)) {
        return Some(r#"쓸 수 없는 글자가 있음 (\ / : * ? " < > |)"#);
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Some("끝에 점이나 공백이 있음");
    }
    let base = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let numbered = |p: &str| {
        base.strip_prefix(p)
            .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
    };
    if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL") || numbered("COM") || numbered("LPT")
    {
        return Some("Windows 예약 이름 (CON·PRN·AUX·NUL·COM1~9·LPT1~9)");
    }
    if name.encode_utf16().count() > 255 {
        return Some("이름이 너무 김 (255자 초과)");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::invalid_name;

    #[test]
    fn windows_name_rules() {
        for bad in [
            "", "..", "a:b", "a/b", "a\\b", "a?", "a*", "a\"b", "a<b", "a|b", "a\u{1}", "a.", "a ",
            "CON", "con.txt", "Com1.log", "LPT9", "nul .txt",
        ] {
            assert!(invalid_name(bad).is_some(), "{bad:?}");
        }
        for ok in [
            "a.txt",
            "CONSOLE.txt",
            "COM10",
            "COM0",
            ".gitignore",
            "사진 1.jpg",
        ] {
            assert_eq!(invalid_name(ok), None, "{ok:?}");
        }
        assert!(invalid_name(&"가".repeat(256)).is_some());
        assert_eq!(invalid_name(&"가".repeat(255)), None);
    }
}
