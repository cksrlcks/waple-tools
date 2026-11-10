//! 파일별 실행. 배치 엔진(원본 이동·백업)은 Phase 1 — 지금은 결과가 job_dir에 남는다.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use localtools_core::{JobContext, Tool, ToolOutput};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileResult {
    /// `outputs`는 절대 경로
    Done {
        input: PathBuf,
        outputs: Vec<PathBuf>,
    },
    Failed {
        input: PathBuf,
        message: String,
    },
    Cancelled {
        input: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub file_index: usize,
    pub total: usize,
    pub ratio: f32,
}

const PANIC_MESSAGE: &str = "도구 실행 중 오류가 발생했습니다.";

/// 입력 파일마다 `job_dir/<index>/`를 out_dir로 주고 도구를 한 번씩 실행한다.
/// 같은 이름 파일이 섞여도 덮어쓰지 않고, 한 파일의 실패·패닉이 나머지를 멈추지 않는다.
pub fn run_files(
    tool: &dyn Tool,
    inputs: &[PathBuf],
    job_dir: &Path,
    opts: &Value,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(Progress) + Send + Sync + 'static,
) -> Vec<FileResult> {
    let total = inputs.len();
    let on_progress = Arc::new(on_progress);
    inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            if cancel.load(Ordering::Relaxed) {
                return FileResult::Cancelled {
                    input: input.clone(),
                };
            }
            let out_dir = job_dir.join(i.to_string());
            let report = on_progress.clone();
            let ctx = JobContext::new(
                move |ratio| {
                    report(Progress {
                        file_index: i,
                        total,
                        ratio,
                    })
                },
                cancel.clone(),
            );
            let result = std::fs::create_dir_all(&out_dir)
                .map_err(|e| e.into())
                .and_then(|_| {
                    catch_unwind(AssertUnwindSafe(|| {
                        tool.run(&ctx, std::slice::from_ref(input), &out_dir, opts)
                    }))
                    .unwrap_or_else(|_| {
                        Err(localtools_core::ToolError::Failed(PANIC_MESSAGE.into()))
                    })
                });
            match result {
                Ok(ToolOutput::Files(files)) => FileResult::Done {
                    input: input.clone(),
                    outputs: files.into_iter().map(|f| out_dir.join(f)).collect(),
                },
                // ponytail: Interactive 블록은 Phase 2 run_interactive에서. 지금은 파일 없음으로 처리
                Ok(ToolOutput::Blocks(_)) => FileResult::Done {
                    input: input.clone(),
                    outputs: vec![],
                },
                Err(localtools_core::ToolError::Cancelled) => FileResult::Cancelled {
                    input: input.clone(),
                },
                Err(e) => FileResult::Failed {
                    input: input.clone(),
                    message: e.to_string(),
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn dummy() -> Box<dyn Tool> {
        localtools_tool_dummy_copy::tool()
    }

    fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, body).unwrap();
        p
    }

    fn no_cancel() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    struct Panicky;
    impl Tool for Panicky {
        fn meta(&self) -> ToolMeta {
            dummy().meta()
        }
        fn kind(&self) -> ToolKind {
            ToolKind::PerFile
        }
        fn options_schema(&self) -> serde_json::Value {
            json!({})
        }
        fn run(
            &self,
            _: &JobContext,
            input: &[PathBuf],
            _: &Path,
            _: &serde_json::Value,
        ) -> Result<ToolOutput, ToolError> {
            if input[0].ends_with("boom.txt") {
                panic!("boom");
            }
            Ok(ToolOutput::Files(vec![]))
        }
    }

    #[test]
    fn same_name_files_do_not_collide() {
        let d = tempfile::tempdir().unwrap();
        let a = write(d.path(), "a/x.txt", "1");
        let b = write(d.path(), "b/x.txt", "2");
        let r = run_files(
            &*dummy(),
            &[a, b],
            &d.path().join("job"),
            &json!({}),
            no_cancel(),
            |_| {},
        );
        let bodies: Vec<String> = r
            .iter()
            .map(|f| match f {
                FileResult::Done { outputs, .. } => std::fs::read_to_string(&outputs[0]).unwrap(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(bodies, ["1", "2"]);
    }

    #[test]
    fn missing_input_fails_only_that_file() {
        let d = tempfile::tempdir().unwrap();
        let a = write(d.path(), "a.txt", "1");
        let c = write(d.path(), "c.txt", "3");
        let r = run_files(
            &*dummy(),
            &[a, d.path().join("없음.txt"), c],
            &d.path().join("job"),
            &json!({}),
            no_cancel(),
            |_| {},
        );
        assert!(matches!(r[0], FileResult::Done { .. }));
        assert!(matches!(&r[1], FileResult::Failed { message, .. } if !message.is_empty()));
        assert!(matches!(r[2], FileResult::Done { .. }));
    }

    #[test]
    fn panic_becomes_failed() {
        let d = tempfile::tempdir().unwrap();
        let a = write(d.path(), "boom.txt", "1");
        let b = write(d.path(), "ok.txt", "2");
        let r = run_files(
            &Panicky,
            &[a, b],
            &d.path().join("job"),
            &json!({}),
            no_cancel(),
            |_| {},
        );
        assert!(
            matches!(&r[0], FileResult::Failed { message, .. } if message == "도구 실행 중 오류가 발생했습니다.")
        );
        assert!(matches!(r[1], FileResult::Done { .. }));
    }

    #[test]
    fn cancel_stops_remaining_files() {
        let d = tempfile::tempdir().unwrap();
        let files: Vec<PathBuf> = ["a", "b", "c"]
            .iter()
            .map(|n| write(d.path(), n, n))
            .collect();
        let cancel = no_cancel();
        let c = cancel.clone();
        let r = run_files(
            &*dummy(),
            &files,
            &d.path().join("job"),
            &json!({}),
            cancel,
            move |p| {
                if p.file_index == 0 {
                    c.store(true, Ordering::Relaxed);
                }
            },
        );
        assert!(matches!(r[0], FileResult::Done { .. }));
        assert!(matches!(r[1], FileResult::Cancelled { .. }));
        assert!(matches!(r[2], FileResult::Cancelled { .. }));
    }
}
