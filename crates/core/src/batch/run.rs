//! 파일별 병렬 실행 → 크기 비교 → 커밋(원본 백업·결과 이동) → 기록.

use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rayon::prelude::*;

use super::collect::{BACKUP_DIR, TMP_PREFIX, plan};
use super::fsops::{create_new_dir, file_hash, move_new, move_new_logged};
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
    on_progress: impl Fn(Progress) + Send + Sync + 'static,
) -> io::Result<JobReport> {
    let plan = plan(tool, spec);
    let log = JobLog::create(log_dir, job_id)?;
    if tool.kind() == ToolKind::Combine {
        let tracker = Tracker::new(1, on_progress);
        let ctx = tracker.context(0, &cancel);
        let files = combine(tool, spec, &plan.files, job_id, &log, &ctx)
            .into_iter()
            .collect();
        tracker.finish(0);
        return Ok(JobReport {
            job_id: job_id.to_string(),
            files,
            skipped: plan.skipped,
        });
    }
    let tracker = Tracker::new(plan.files.len(), on_progress);
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
                let ctx = tracker.context(i, &cancel);
                let result = process(tool, spec, f, &tmp, &log, &ctx);
                let _ = std::fs::remove_dir_all(&tmp);
                tracker.finish(i);
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

/// 진행률: 끝난 파일 수 + 진행 중인 파일마다 도구가 알려 준 비율. 병렬로 불려도 된다.
struct Tracker {
    parts: Mutex<Vec<f32>>,
    done: AtomicUsize,
    send: Box<dyn Fn(Progress) + Send + Sync>,
}

impl Tracker {
    fn new(total: usize, send: impl Fn(Progress) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            parts: Mutex::new(vec![0.0; total]),
            done: AtomicUsize::new(0),
            send: Box::new(send),
        })
    }

    fn report(&self, i: usize, ratio: f32) {
        let mut parts = self.parts.lock().unwrap_or_else(|e| e.into_inner());
        let Some(slot) = parts.get_mut(i) else {
            return;
        };
        *slot = ratio.clamp(0.0, 1.0);
        let total = parts.len();
        let ratio = parts.iter().sum::<f32>() / total.max(1) as f32;
        // 잠금 안에서 보내야 비율이 거꾸로 가지 않는다
        (self.send)(Progress {
            done: self.done.load(Ordering::Relaxed),
            total,
            ratio,
        });
    }

    /// `i`번째 파일을 처리할 도구에 넘길 통로 (도구의 진행률이 전체 비율에 반영된다)
    fn context(self: &Arc<Self>, i: usize, cancel: &Arc<AtomicBool>) -> JobContext {
        let me = self.clone();
        JobContext::new(move |r| me.report(i, r), cancel.clone())
    }

    fn finish(&self, i: usize) {
        self.done.fetch_add(1, Ordering::Relaxed);
        self.report(i, 1.0);
    }
}

/// 도구를 실행해 임시 폴더 안 결과 이름을 받는다. 오류는 그대로 `FileResult`로.
fn invoke(
    tool: &dyn Tool,
    inputs: &[PathBuf],
    first: &Path,
    tmp: &Path,
    opts: &serde_json::Value,
    ctx: &JobContext,
) -> Result<Vec<PathBuf>, FileResult> {
    let failed = |message: String| FileResult::Failed {
        input: first.to_path_buf(),
        message,
    };
    if let Err(e) = std::fs::create_dir_all(tmp) {
        return Err(failed(ToolError::from(e).to_string()));
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| tool.run(ctx, inputs, tmp, opts)));
    let names = match outcome {
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            eprintln!("[waple-tools] {} 패닉: {msg}", tool.meta().id);
            return Err(failed(PANIC_MESSAGE.into()));
        }
        Ok(Err(ToolError::Cancelled)) => {
            return Err(FileResult::Cancelled {
                input: first.to_path_buf(),
            });
        }
        Ok(Err(e)) => return Err(failed(e.to_string())),
        Ok(Ok(ToolOutput::Blocks(_))) => {
            return Err(failed("지원하지 않는 도구 종류입니다.".into()));
        }
        Ok(Ok(ToolOutput::Files(names))) => names,
    };
    if names.is_empty() {
        return Err(failed("도구가 결과 파일을 만들지 않았습니다.".into()));
    }
    if !names.iter().all(|n| inside(n) && tmp.join(n).is_file()) {
        return Err(failed("도구가 잘못된 결과 경로를 돌려주었습니다.".into()));
    }
    Ok(names)
}

/// Combine: 모든 파일을 넣은 순서대로 한 번에 실행하고, 결과를 첫 파일의 결과 폴더에 둔다. 원본은 그대로.
fn combine(
    tool: &dyn Tool,
    spec: &JobSpec,
    files: &[PlannedFile],
    job_id: &str,
    log: &JobLog,
    ctx: &JobContext,
) -> Option<FileResult> {
    let first = files.first()?;
    if ctx.is_cancelled() {
        return Some(FileResult::Cancelled {
            input: first.input.clone(),
        });
    }
    let inputs: Vec<PathBuf> = files.iter().map(|f| f.input.clone()).collect();
    let tmp = first.dest_dir.join(format!("{TMP_PREFIX}{job_id}-0"));
    let result = invoke(tool, &inputs, &first.input, &tmp, &spec.opts, ctx).and_then(|names| {
        let size_before = inputs
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        let size_after = sizes(&tmp, &names);
        let placed = if names.iter().all(|n| n.components().count() == 1) {
            commit_files(first, &tmp, &names, log)
        } else {
            commit_set(first, &tmp, &names, log)
        };
        placed
            .map(|(outputs, _)| FileResult::Done {
                input: first.input.clone(),
                outputs,
                backup: None,
                size_before,
                size_after,
            })
            .map_err(|message| FileResult::Failed {
                input: first.input.clone(),
                message,
            })
    });
    let _ = std::fs::remove_dir_all(&tmp);
    Some(result.unwrap_or_else(|r| r))
}

fn sizes(tmp: &Path, names: &[PathBuf]) -> u64 {
    names
        .iter()
        .filter_map(|n| std::fs::metadata(tmp.join(n)).ok())
        .map(|m| m.len())
        .sum()
}

fn process(
    tool: &dyn Tool,
    spec: &JobSpec,
    f: &PlannedFile,
    tmp: &Path,
    log: &JobLog,
    ctx: &JobContext,
) -> FileResult {
    let input = f.input.clone();
    let failed = |message: String| FileResult::Failed {
        input: input.clone(),
        message,
    };
    if ctx.is_cancelled() {
        return FileResult::Cancelled { input };
    }
    let generator = match tool.kind() {
        ToolKind::PerFile => false,
        ToolKind::Generator => true,
        _ => return failed("지원하지 않는 도구 종류입니다.".into()),
    };
    let opts = file_opts(&spec.opts, &input);
    let names = match invoke(tool, std::slice::from_ref(&input), &input, tmp, &opts, ctx) {
        Ok(names) => names,
        Err(r) => return r,
    };
    let size_before = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
    let size_after = sizes(tmp, &names);
    if spec.keep_if_larger && !generator && names.len() == 1 && size_after > size_before {
        return FileResult::Skipped {
            input,
            reason: "용량 증가".into(),
        };
    }

    let committed = if generator {
        commit_set(f, tmp, &names, log)
    } else {
        commit(&spec.policy, f, tmp, &names, log)
    };
    match committed {
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

/// 파일마다 다른 옵션: `opts["x-per-file"]["<입력 경로>"]`의 값을 그 파일에만 덮어쓴다.
/// 키는 입력 파일의 절대 경로(앱이 넣은 경로). 폴더 안 파일처럼 키가 없으면 공통 옵션. `x-per-file` 자체는 도구에 넘기지 않는다.
pub(crate) fn file_opts(opts: &serde_json::Value, input: &Path) -> serde_json::Value {
    let mut out = opts.clone();
    let Some(obj) = out.as_object_mut() else {
        return out;
    };
    let Some(serde_json::Value::Object(per)) = obj.remove("x-per-file") else {
        return out;
    };
    let key = input.to_string_lossy();
    if let Some(serde_json::Value::Object(over)) = per.get(key.as_ref()) {
        for (k, v) in over {
            obj.insert(k.clone(), v.clone());
        }
    }
    out
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

/// Generator: 결과 세트를 `<dest_dir>/<최상위 폴더>`(겹치면 `(1)`)에 둔다. 원본은 그대로 둔다 (저장 방식과 상관없이).
/// 도구는 모든 결과를 폴더 하나 아래에 만든다 (예: `logo-icons/favicon/favicon.ico`).
fn commit_set(
    f: &PlannedFile,
    tmp: &Path,
    names: &[PathBuf],
    log: &JobLog,
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    let io_err = |e: io::Error| ToolError::from(e).to_string();
    let bad = || "도구가 잘못된 결과 경로를 돌려주었습니다.".to_string();
    let top = names[0].components().next().ok_or_else(bad)?;
    let rests: Vec<&Path> = names
        .iter()
        .map(|n| {
            n.strip_prefix(top)
                .ok()
                .filter(|r| inside(r))
                .ok_or_else(bad)
        })
        .collect::<Result<_, _>>()?;

    let dir = create_new_dir(&f.dest_dir.join(top)).map_err(io_err)?;
    if let Err(e) = log.append(&LogEntry::CreatedDir { path: dir.clone() }) {
        let _ = std::fs::remove_dir(&dir);
        return Err(io_err(e));
    }
    let mut outputs = Vec::new();
    for (name, rest) in names.iter().zip(rests) {
        let src = tmp.join(name);
        let placed = file_hash(&src).and_then(|hash| {
            move_new_logged(&src, &dir.join(rest), |p| {
                log.append(&LogEntry::Created {
                    path: p.to_path_buf(),
                    hash,
                })
            })
        });
        match placed {
            Ok(p) => outputs.push(p),
            Err(e) => {
                // 방금 만든 폴더라 통째로 지워도 남의 파일이 없다
                let _ = std::fs::remove_dir_all(&dir);
                return Err(io_err(e));
            }
        }
    }
    Ok((outputs, None))
}

/// Combine 결과 파일들을 `<dest_dir>/<이름>`(겹치면 `(1)`)에 둔다. 원본은 그대로. 실패하면 이미 옮긴 결과를 지운다.
fn commit_files(
    f: &PlannedFile,
    tmp: &Path,
    names: &[PathBuf],
    log: &JobLog,
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    let io_err = |e: io::Error| ToolError::from(e).to_string();
    std::fs::create_dir_all(&f.dest_dir).map_err(io_err)?;
    let mut outputs = Vec::new();
    for name in names {
        let src = tmp.join(name);
        let placed = file_hash(&src).and_then(|hash| {
            move_new_logged(&src, &f.dest_dir.join(name), |p| {
                log.append(&LogEntry::Created {
                    path: p.to_path_buf(),
                    hash,
                })
            })
        });
        match placed {
            Ok(p) => outputs.push(p),
            Err(e) => {
                for p in &outputs {
                    let _ = std::fs::remove_file(p);
                }
                return Err(io_err(e));
            }
        }
    }
    Ok((outputs, None))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn per_file_options_override_only_that_file() {
        let opts = json!({
            "format": "woff2",
            "x-per-file": { "/f/a.woff2": { "format": "sfnt" } }
        });
        assert_eq!(
            file_opts(&opts, Path::new("/f/a.woff2")),
            json!({ "format": "sfnt" })
        );
        assert_eq!(
            file_opts(&opts, Path::new("/f/b.ttf")),
            json!({ "format": "woff2" })
        );
        assert_eq!(file_opts(&json!(null), Path::new("x")), json!(null));
    }
}
