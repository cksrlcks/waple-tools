//! 파일별 병렬 실행 → 크기 비교 → 커밋(원본 백업·결과 이동) → 기록.

use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use super::collect::{BACKUP_DIR, TMP_PREFIX, plan};
use super::fsops::{file_hash, move_new, move_new_logged};
use super::log::{JobLog, LogEntry};
use super::{FileResult, JobReport, JobSpec, OutputPolicy, PlannedFile, Progress};
use crate::{JobContext, Tool, ToolError, ToolKind, ToolOutput};

pub(crate) const PANIC_MESSAGE: &str = "도구 실행 중 오류가 발생했습니다.";

/// `threads`가 0이면 CPU 코어 수 - 1 (최소 1)
pub fn effective_threads(threads: usize) -> usize {
    if threads > 0 {
        return threads;
    }
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1))
        .unwrap_or(1)
        .max(1)
}

/// 작업 실행. `io::Error`는 기록 파일을 만들 수 없을 때만 — 파일 단위 실패는 결과에 담긴다.
pub fn run(
    tool: &dyn Tool,
    spec: &JobSpec,
    job_id: &str,
    log_dir: &Path,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(Progress) + Send + Sync,
) -> io::Result<JobReport> {
    let plan = plan(tool, spec);
    let log = JobLog::create(log_dir, job_id)?;
    let total = plan.files.len();
    let done = AtomicUsize::new(0);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(effective_threads(spec.threads))
        .build()
        .map_err(io::Error::other)?;
    let files = pool.install(|| {
        plan.files
            .par_iter()
            .enumerate()
            .map(|(i, f)| {
                let tmp = f.dest_dir.join(format!("{TMP_PREFIX}{job_id}-{i}"));
                let result = process(tool, spec, f, &tmp, &log, &cancel);
                let _ = std::fs::remove_dir_all(&tmp);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                on_progress(Progress { done: n, total });
                result
            })
            .collect()
    });
    Ok(JobReport {
        job_id: job_id.to_string(),
        files,
        skipped: plan.skipped,
    })
}

fn process(
    tool: &dyn Tool,
    spec: &JobSpec,
    f: &PlannedFile,
    tmp: &Path,
    log: &JobLog,
    cancel: &Arc<AtomicBool>,
) -> FileResult {
    let input = f.input.clone();
    let failed = |message: String| FileResult::Failed {
        input: input.clone(),
        message,
    };
    if cancel.load(Ordering::Relaxed) {
        return FileResult::Cancelled { input };
    }
    if tool.kind() != ToolKind::PerFile {
        return failed("지원하지 않는 도구 종류입니다.".into());
    }
    if let Err(e) = std::fs::create_dir_all(tmp) {
        return failed(ToolError::from(e).to_string());
    }

    let ctx = JobContext::new(|_| {}, cancel.clone());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tool.run(&ctx, std::slice::from_ref(&input), tmp, &spec.opts)
    }));
    let names = match outcome {
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            eprintln!("[waple-tools] {} 패닉: {msg}", tool.meta().id);
            return failed(PANIC_MESSAGE.into());
        }
        Ok(Err(ToolError::Cancelled)) => return FileResult::Cancelled { input },
        Ok(Err(e)) => return failed(e.to_string()),
        Ok(Ok(ToolOutput::Blocks(_))) => return failed("지원하지 않는 도구 종류입니다.".into()),
        Ok(Ok(ToolOutput::Files(names))) => names,
    };

    if names.is_empty() {
        return failed("도구가 결과 파일을 만들지 않았습니다.".into());
    }
    if !names.iter().all(|n| inside(n) && tmp.join(n).is_file()) {
        return failed("도구가 잘못된 결과 경로를 돌려주었습니다.".into());
    }
    let size_before = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
    let size_after: u64 = names
        .iter()
        .filter_map(|n| std::fs::metadata(tmp.join(n)).ok())
        .map(|m| m.len())
        .sum();
    if spec.keep_if_larger && names.len() == 1 && size_after > size_before {
        return FileResult::Skipped {
            input,
            reason: "용량 증가".into(),
        };
    }

    match commit(&spec.policy, f, tmp, &names, log) {
        Ok((outputs, backup)) => FileResult::Done {
            input,
            outputs,
            backup,
            size_before,
            size_after,
        },
        Err(message) => failed(message),
    }
}

/// 비어 있지 않은 상대 경로이고 `..`·루트·드라이브 없이 이름만으로 이루어졌는가
pub(crate) fn inside(name: &Path) -> bool {
    name.components().next().is_some()
        && name.components().all(|c| matches!(c, Component::Normal(_)))
}

/// 결과를 최종 위치로 옮기고 기록한다. 실패하면 이미 옮긴 결과를 지우고 원본을 되돌린다.
fn commit(
    policy: &OutputPolicy,
    f: &PlannedFile,
    tmp: &Path,
    names: &[PathBuf],
    log: &JobLog,
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    let io_err = |e: io::Error| ToolError::from(e).to_string();
    let backup = match policy {
        OutputPolicy::InPlaceWithBackup => {
            let hash = file_hash(&f.input).map_err(io_err)?;
            let want = f.root.join(BACKUP_DIR).join(&f.rel);
            let to = move_new_logged(&f.input, &want, |to| {
                log.append(&LogEntry::Moved {
                    from: f.input.clone(),
                    to: to.to_path_buf(),
                    hash,
                })
            })
            .map_err(io_err)?;
            Some(to)
        }
        OutputPolicy::KeepOriginal { .. } | OutputPolicy::OutputFolder { .. } => None,
    };

    let mut outputs = Vec::new();
    for name in names {
        let target = match policy {
            OutputPolicy::KeepOriginal { suffix } => f.dest_dir.join(with_suffix(name, suffix)),
            _ => f.dest_dir.join(name),
        };
        let src = tmp.join(name);
        let placed = file_hash(&src).and_then(|hash| {
            move_new_logged(&src, &target, |p| {
                log.append(&LogEntry::Created {
                    path: p.to_path_buf(),
                    hash,
                })
            })
        });
        match placed {
            Ok(p) => outputs.push(p),
            Err(e) => {
                // ponytail: 되돌린 동작은 기록에 남아 있지만, 되돌리기 때 "이미 없음"으로 건너뛰어진다
                for p in &outputs {
                    let _ = std::fs::remove_file(p);
                }
                if let Some(b) = &backup {
                    let _ = move_new(b, &f.input);
                }
                return Err(io_err(e));
            }
        }
    }
    Ok((outputs, backup))
}

/// `a.png` + `_copy` → `a_copy.png` (하위 경로가 있으면 마지막 이름만 바꾼다)
fn with_suffix(name: &Path, suffix: &str) -> PathBuf {
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let file = match name.extension() {
        Some(ext) => format!("{stem}{suffix}.{}", ext.to_string_lossy()),
        None => format!("{stem}{suffix}"),
    };
    name.with_file_name(file)
}
