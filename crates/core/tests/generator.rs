//! Generator(결과 세트 폴더)·Interactive(결과 블록) 실행. 임시 폴더에서 실제 파일로 확인한다.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use waple_tools_core::batch::{
    FileResult, JobReport, JobSpec, OutputPolicy, interactive, run, undo,
};
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

/// `<stem>-set/a.txt`, `<stem>-set/sub/b.txt`를 만든다. `two_tops`면 최상위 폴더가 둘, `flat`이면 폴더 없이 `<stem>-shot.txt` 하나
#[derive(Default)]
struct SetTool {
    two_tops: bool,
    flat: bool,
}

impl Tool for SetTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "test.set".into(),
            name: "테스트 세트".into(),
            category: "test".into(),
            description: String::new(),
            extensions: vec![],
        }
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Generator
    }
    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    fn run(
        &self,
        _: &JobContext,
        input: &[PathBuf],
        out: &Path,
        _: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let stem = input[0].file_stem().unwrap().to_string_lossy().to_string();
        if self.flat {
            let name = format!("{stem}-shot.txt");
            std::fs::write(out.join(&name), "한 장")?;
            return Ok(ToolOutput::Files(vec![PathBuf::from(name)]));
        }
        let top = format!("{stem}-set");
        let other = if self.two_tops { "other" } else { top.as_str() };
        let names = [format!("{top}/a.txt"), format!("{other}/sub/b.txt")];
        for n in &names {
            let p = out.join(n);
            std::fs::create_dir_all(p.parent().unwrap())?;
            std::fs::write(p, n)?;
        }
        Ok(ToolOutput::Files(names.iter().map(PathBuf::from).collect()))
    }
}

fn spec(input: &Path, policy: OutputPolicy) -> JobSpec {
    JobSpec {
        inputs: vec![input.to_path_buf()],
        recursive: false,
        policy,
        keep_if_larger: true,
        threads: 1,
        opts: serde_json::json!({}),
    }
}

fn go(tool: &SetTool, spec: &JobSpec, logs: &Path, id: &str) -> JobReport {
    run(
        tool,
        spec,
        id,
        logs,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .unwrap()
}

#[test]
fn set_goes_next_to_input_and_keeps_original() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("work"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let src = work.join("logo.png");
    std::fs::write(&src, "원본").unwrap();

    // 기본 저장 방식(원본 자리 + 백업)이어도 Generator는 원본을 옮기지 않는다
    let rep = go(
        &SetTool::default(),
        &spec(&src, OutputPolicy::default()),
        &logs,
        "j1",
    );
    assert!(
        matches!(&rep.files[0], FileResult::Done { outputs, backup: None, .. } if outputs.len() == 2)
    );
    assert_eq!(std::fs::read_to_string(&src).unwrap(), "원본");
    assert!(!work.join("original").exists());
    assert_eq!(
        std::fs::read_to_string(work.join("logo-set/sub/b.txt")).unwrap(),
        "logo-set/sub/b.txt"
    );

    // 같은 이름 폴더가 있으면 번호를 붙인다
    go(
        &SetTool::default(),
        &spec(&src, OutputPolicy::default()),
        &logs,
        "j2",
    );
    assert!(work.join("logo-set (1)/a.txt").is_file());

    // 되돌리면 폴더째 사라지고 다른 실행의 결과는 남는다
    undo(&logs, "j2").unwrap();
    assert!(!work.join("logo-set (1)").exists());
    assert!(work.join("logo-set/a.txt").is_file());
    undo(&logs, "j1").unwrap();
    assert!(!work.join("logo-set").exists());
    assert_eq!(std::fs::read_to_string(&src).unwrap(), "원본");
}

#[test]
fn set_respects_output_folder() {
    let d = tempfile::tempdir().unwrap();
    let (work, out, logs) = (
        d.path().join("w"),
        d.path().join("out"),
        d.path().join("logs"),
    );
    std::fs::create_dir(&work).unwrap();
    let src = work.join("logo.png");
    std::fs::write(&src, "원본").unwrap();
    go(
        &SetTool::default(),
        &spec(&src, OutputPolicy::OutputFolder { dir: out.clone() }),
        &logs,
        "j1",
    );
    assert!(out.join("logo-set/sub/b.txt").is_file());
    assert!(!work.join("logo-set").exists());
}

#[test]
fn set_must_have_one_top_folder() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("w"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let src = work.join("logo.png");
    std::fs::write(&src, "원본").unwrap();
    let tool = SetTool {
        two_tops: true,
        ..Default::default()
    };
    let rep = go(&tool, &spec(&src, OutputPolicy::default()), &logs, "j1");
    assert!(matches!(&rep.files[0], FileResult::Failed { .. }));
    assert!(!work.join("logo-set").exists());
}

/// 폴더 없는 결과는 원본 옆에 파일로 (백업 폴더 없음, 겹치면 번호)
#[test]
fn flat_result_goes_next_to_input() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("w"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let src = work.join("clip.mp4");
    std::fs::write(&src, "원본").unwrap();
    let tool = SetTool {
        flat: true,
        ..Default::default()
    };
    let rep = go(&tool, &spec(&src, OutputPolicy::default()), &logs, "j1");
    assert!(
        matches!(&rep.files[0], FileResult::Done { outputs, backup: None, .. } if outputs == &vec![work.join("clip-shot.txt")])
    );
    assert_eq!(std::fs::read_to_string(&src).unwrap(), "원본");
    assert!(!work.join("original").exists());
    go(&tool, &spec(&src, OutputPolicy::default()), &logs, "j2");
    assert!(work.join("clip-shot (1).txt").is_file());

    undo(&logs, "j2").unwrap();
    assert!(!work.join("clip-shot (1).txt").exists());
    assert!(work.join("clip-shot.txt").is_file());
    undo(&logs, "j1").unwrap();
    assert!(!work.join("clip-shot.txt").exists());
    assert_eq!(std::fs::read_to_string(&src).unwrap(), "원본");
}

/// 사용자가 결과 폴더에 파일을 넣었으면 되돌리기 때 폴더를 남긴다
#[test]
fn undo_keeps_set_folder_with_user_files() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("w"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let src = work.join("logo.png");
    std::fs::write(&src, "원본").unwrap();
    go(
        &SetTool::default(),
        &spec(&src, OutputPolicy::default()),
        &logs,
        "j1",
    );
    std::fs::write(work.join("logo-set/mine.txt"), "내 파일").unwrap();
    undo(&logs, "j1").unwrap();
    assert!(work.join("logo-set/mine.txt").is_file());
    assert!(!work.join("logo-set/a.txt").exists());
}

struct EchoTool;

impl Tool for EchoTool {
    fn meta(&self) -> ToolMeta {
        SetTool::default().meta()
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }
    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    fn run(
        &self,
        _: &JobContext,
        input: &[PathBuf],
        _: &Path,
        _: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let text = match input.first() {
            Some(p) => std::fs::read_to_string(p)?,
            None => "입력 없음".into(),
        };
        Ok(ToolOutput::Blocks(vec![Block::Text { text }]))
    }
}

#[test]
fn interactive_returns_blocks_and_writes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.txt");
    std::fs::write(&src, "안녕").unwrap();
    let blocks = interactive(&EchoTool, Some(&src), &serde_json::json!({})).unwrap();
    assert!(matches!(&blocks[..], [Block::Text { text }] if text == "안녕"));
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    // 파일을 만드는 도구는 Interactive로 실행하지 않는다
    assert!(interactive(&SetTool::default(), Some(&src), &serde_json::json!({})).is_err());
    // 입력 파일 없이 (`x-input: "none"`) 실행하면 빈 입력 목록을 받는다
    let blocks = interactive(&EchoTool, None, &serde_json::json!({})).unwrap();
    assert!(matches!(&blocks[..], [Block::Text { text }] if text == "입력 없음"));
}
