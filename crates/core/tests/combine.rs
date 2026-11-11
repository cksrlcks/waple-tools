//! Combine(여러 파일 → 결과 하나) 실행. 임시 폴더에서 실제 파일로 확인한다.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use waple_tools_core::batch::{FileResult, JobReport, JobSpec, OutputPolicy, run, undo};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

/// 입력 내용을 순서대로 이어 붙여 `<첫 이름>-joined.txt` 하나를 만든다
struct JoinTool;

impl Tool for JoinTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "test.join".into(),
            name: "테스트 합치기".into(),
            category: "test".into(),
            description: String::new(),
            extensions: vec!["txt".into()],
        }
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Combine
    }
    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        if opts["fail"] == true {
            return Err(ToolError::Failed("실패".into()));
        }
        ctx.progress(0.5);
        let mut text = String::new();
        for p in input {
            text += &std::fs::read_to_string(p)?;
        }
        let stem = input[0].file_stem().unwrap().to_string_lossy();
        let name = format!("{stem}-joined.txt");
        std::fs::write(out.join(&name), text)?;
        Ok(ToolOutput::Files(vec![name.into()]))
    }
}

fn spec(inputs: Vec<PathBuf>, policy: OutputPolicy) -> JobSpec {
    JobSpec {
        inputs,
        recursive: false,
        policy,
        keep_if_larger: true,
        threads: 4,
        opts: serde_json::json!({}),
    }
}

fn go(spec: &JobSpec, logs: &Path, id: &str) -> JobReport {
    run(
        &JoinTool,
        spec,
        id,
        logs,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .unwrap()
}

#[test]
fn joins_in_given_order_and_keeps_originals() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("work"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    for (n, t) in [("a", "A"), ("b", "B"), ("c", "C")] {
        std::fs::write(work.join(format!("{n}.txt")), t).unwrap();
    }
    let inputs = ["c", "a", "b"]
        .map(|n| work.join(format!("{n}.txt")))
        .to_vec();

    // 기본 저장 방식(원본 자리 + 백업)이어도 원본은 옮기지 않는다
    let rep = go(&spec(inputs.clone(), OutputPolicy::default()), &logs, "j1");
    assert_eq!(rep.files.len(), 1, "결과 하나");
    let FileResult::Done {
        input,
        outputs,
        backup,
        size_before,
        ..
    } = &rep.files[0]
    else {
        panic!("{:?}", rep.files[0]);
    };
    assert_eq!(input, &inputs[0]);
    assert_eq!(*size_before, 3);
    assert!(backup.is_none());
    assert_eq!(outputs, &vec![work.join("c-joined.txt")]);
    assert_eq!(std::fs::read_to_string(&outputs[0]).unwrap(), "CAB");
    assert!(!work.join("original").exists());
    for p in &inputs {
        assert!(p.is_file());
    }

    // 이름이 겹치면 번호를 붙이고, 되돌리면 그 결과만 지운다
    go(&spec(inputs.clone(), OutputPolicy::default()), &logs, "j2");
    assert!(work.join("c-joined (1).txt").is_file());
    undo(&logs, "j2").unwrap();
    assert!(!work.join("c-joined (1).txt").exists());
    assert!(work.join("c-joined.txt").is_file());
    // 임시 폴더가 남지 않는다
    let left: Vec<_> = std::fs::read_dir(&work)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('.'))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn output_folder_and_folder_input_sorted_by_name() {
    let d = tempfile::tempdir().unwrap();
    let (work, out, logs) = (
        d.path().join("work"),
        d.path().join("out"),
        d.path().join("logs"),
    );
    std::fs::create_dir(&work).unwrap();
    std::fs::write(work.join("2.txt"), "2").unwrap();
    std::fs::write(work.join("1.txt"), "1").unwrap();
    let rep = go(
        &spec(
            vec![work.clone()],
            OutputPolicy::OutputFolder { dir: out.clone() },
        ),
        &logs,
        "j1",
    );
    assert!(matches!(&rep.files[0], FileResult::Done { .. }), "{rep:?}");
    assert_eq!(
        std::fs::read_to_string(out.join("1-joined.txt")).unwrap(),
        "12"
    );
}

#[test]
fn failure_is_reported_once_and_leaves_nothing() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("work"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    std::fs::write(work.join("a.txt"), "A").unwrap();
    let mut s = spec(vec![work.join("a.txt")], OutputPolicy::default());
    s.opts = serde_json::json!({ "fail": true });
    let rep = go(&s, &logs, "j1");
    assert!(
        matches!(&rep.files[..], [FileResult::Failed { message, .. }] if message == "실패"),
        "{rep:?}"
    );
    assert_eq!(std::fs::read_dir(&work).unwrap().count(), 1);
}

#[test]
fn nothing_to_do_gives_empty_report() {
    let d = tempfile::tempdir().unwrap();
    let rep = go(
        &spec(vec![d.path().join("none.txt")], OutputPolicy::default()),
        &d.path().join("logs"),
        "j1",
    );
    assert!(rep.files.is_empty());
    assert_eq!(rep.skipped.len(), 1);
}

#[test]
fn tool_progress_is_forwarded_as_ratio() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.txt");
    std::fs::write(&src, "A").unwrap();
    let seen = Arc::new(std::sync::Mutex::new(vec![]));
    let s = seen.clone();
    run(
        &JoinTool,
        &spec(vec![src], OutputPolicy::default()),
        "j1",
        &d.path().join("logs"),
        Arc::new(AtomicBool::new(false)),
        move |p| s.lock().unwrap().push((p.done, p.total, p.ratio)),
    )
    .unwrap();
    assert_eq!(*seen.lock().unwrap(), [(0, 1, 0.5), (1, 1, 1.0)]);
}
