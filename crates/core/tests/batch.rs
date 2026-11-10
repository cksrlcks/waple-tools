//! 배치 엔진 통합 테스트. 모두 임시 폴더에서 실제 파일로 확인한다.

#[allow(dead_code)] // 도구 대역의 일부 필드·헬퍼는 특정 테스트에서만 쓴다
mod support {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use localtools_core::batch::{JobSpec, OutputPolicy};
    use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

    /// 입력을 복사하는 도구 대역. 필드로 실패·패닉·크기 증가·확장자 변경·잘못된 경로를 흉내 낸다.
    #[derive(Default)]
    pub struct CopyTool {
        pub exts: Vec<&'static str>,
        pub grow: bool,
        pub fail_on: Option<&'static str>,
        pub panic_on: Option<&'static str>,
        pub rename_to_ext: Option<&'static str>,
        pub escape: bool,
    }

    pub fn png_tool() -> CopyTool {
        CopyTool {
            exts: vec!["png"],
            ..Default::default()
        }
    }

    impl Tool for CopyTool {
        fn meta(&self) -> ToolMeta {
            ToolMeta {
                id: "test.copy".into(),
                name: "테스트 복사".into(),
                category: "test".into(),
                description: String::new(),
                extensions: self.exts.iter().map(|e| e.to_string()).collect(),
            }
        }
        fn kind(&self) -> ToolKind {
            ToolKind::PerFile
        }
        fn options_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        fn run(
            &self,
            ctx: &JobContext,
            input: &[PathBuf],
            out_dir: &Path,
            _: &serde_json::Value,
        ) -> Result<ToolOutput, ToolError> {
            ctx.check_cancel()?;
            let src = &input[0];
            let stem = src.file_stem().unwrap().to_string_lossy().to_string();
            if self.fail_on == Some(stem.as_str()) {
                return Err(ToolError::Failed("일부러 실패".into()));
            }
            if self.panic_on == Some(stem.as_str()) {
                panic!("일부러 패닉");
            }
            if self.escape {
                return Ok(ToolOutput::Files(vec![PathBuf::from("../x.png")]));
            }
            let name = match self.rename_to_ext {
                Some(ext) => format!("{stem}.{ext}"),
                None => src.file_name().unwrap().to_string_lossy().to_string(),
            };
            let mut body = std::fs::read(src)?;
            if self.grow {
                body.extend_from_slice(b"-bigger");
            }
            std::fs::write(out_dir.join(&name), body)?;
            Ok(ToolOutput::Files(vec![PathBuf::from(name)]))
        }
    }

    pub fn write(root: &Path, rel: &str, body: &str) -> PathBuf {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, body).unwrap();
        p
    }

    pub fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap()
    }

    /// 폴더 트리 스냅샷: 상대 경로(`/` 구분) → 내용. 폴더는 `이름/` 키에 빈 값.
    pub fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                let rel = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if p.is_dir() {
                    out.insert(format!("{rel}/"), vec![]);
                    walk(root, &p, out);
                } else {
                    out.insert(rel, std::fs::read(&p).unwrap());
                }
            }
        }
        walk(root, root, &mut out);
        out
    }

    pub fn spec(inputs: Vec<PathBuf>, recursive: bool, policy: OutputPolicy) -> JobSpec {
        JobSpec {
            inputs,
            recursive,
            policy,
            keep_if_larger: true,
            threads: 2,
            opts: serde_json::json!({}),
        }
    }
}

use std::path::PathBuf;

use localtools_core::batch::{OutputPolicy, plan};
use support::*;

#[test]
fn plan_excludes_original_hidden_tmp_and_unsupported() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    for f in [
        "a.png",
        "b.txt",
        "original/c.png",
        ".git/d.png",
        ".localtools-tmp-1/e.png",
        "sub/f.png",
    ] {
        write(r, f, "x");
    }
    let tool = png_tool();

    let p = plan(&tool, &spec(vec![r.into()], false, OutputPolicy::default()));
    let files: Vec<_> = p.files.iter().map(|f| f.rel.clone()).collect();
    assert_eq!(files, [PathBuf::from("a.png")]);
    assert_eq!(p.skipped.len(), 1);
    assert_eq!(p.skipped[0].path, r.join("b.txt"));
    assert_eq!(p.skipped[0].reason, "지원하지 않는 형식");

    let p = plan(&tool, &spec(vec![r.into()], true, OutputPolicy::default()));
    let files: Vec<_> = p.files.iter().map(|f| f.rel.clone()).collect();
    assert_eq!(files, [PathBuf::from("a.png"), PathBuf::from("sub/f.png")]);
}

#[test]
fn plan_single_file_root_is_parent() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "dir/a.png", "x");
    let p = plan(
        &png_tool(),
        &spec(vec![a.clone()], false, OutputPolicy::default()),
    );
    assert_eq!(p.files.len(), 1);
    assert_eq!(p.files[0].input, a);
    assert_eq!(p.files[0].root, d.path().join("dir"));
    assert_eq!(p.files[0].rel, PathBuf::from("a.png"));
    assert_eq!(p.files[0].dest_dir, d.path().join("dir"));
}

#[test]
fn plan_output_folder_keeps_structure() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("src");
    write(&src, "sub/f.png", "x");
    let out = d.path().join("out");
    let p = plan(
        &png_tool(),
        &spec(
            vec![src.clone()],
            true,
            OutputPolicy::OutputFolder { dir: out.clone() },
        ),
    );
    assert_eq!(p.files[0].dest_dir, out.join("sub"));
}

#[test]
fn plan_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.png", "x");
    write(d.path(), "sub/b.png", "y");
    let before = tree(d.path());
    plan(
        &png_tool(),
        &spec(vec![d.path().into()], true, OutputPolicy::default()),
    );
    assert_eq!(tree(d.path()), before);
}

#[test]
fn plan_dedups_same_file() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.png", "x");
    let p = plan(
        &png_tool(),
        &spec(vec![d.path().into(), a], false, OutputPolicy::default()),
    );
    assert_eq!(p.files.len(), 1);
}

#[test]
fn plan_reports_missing_path() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("없음.png");
    let p = plan(
        &png_tool(),
        &spec(vec![missing.clone()], false, OutputPolicy::default()),
    );
    assert!(p.files.is_empty());
    assert_eq!(p.skipped[0].path, missing);
    assert_eq!(p.skipped[0].reason, "파일을 찾을 수 없음");
}

// ---------- 실행 (InPlaceWithBackup) ----------

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use localtools_core::batch::{FileResult, JobReport, LogEntry, read_log, run};

fn run_with(tool: &CopyTool, spec: &localtools_core::batch::JobSpec, log_dir: &Path) -> JobReport {
    run(
        tool,
        spec,
        "job1",
        log_dir,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .unwrap()
}

fn no_tmp_dirs(root: &Path) -> bool {
    !tree(root).keys().any(|k| k.contains(".localtools-tmp-"))
}

#[test]
fn in_place_backs_up_and_replaces() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    write(&r, "sub/b.png", "B");
    let rep = run_with(
        &png_tool(),
        &spec(vec![r.clone()], true, OutputPolicy::default()),
        &logs,
    );
    assert!(
        rep.files
            .iter()
            .all(|f| matches!(f, FileResult::Done { .. })),
        "{rep:?}"
    );
    assert_eq!(read(&r.join("a.png")), "A");
    assert_eq!(read(&r.join("original/a.png")), "A");
    assert_eq!(read(&r.join("original/sub/b.png")), "B");
    assert!(no_tmp_dirs(&r));
    let log = read_log(&logs, "job1").unwrap();
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, LogEntry::Moved { .. }))
            .count(),
        2
    );
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, LogEntry::Created { .. }))
            .count(),
        2
    );
}

#[test]
fn backup_name_collision_gets_number() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    write(&r, "original/a.png", "old");
    let rep = run_with(
        &png_tool(),
        &spec(vec![r.clone()], false, OutputPolicy::default()),
        &logs,
    );
    assert!(
        matches!(&rep.files[0], FileResult::Done { backup: Some(b), .. } if b == &r.join("original/a (1).png"))
    );
    assert_eq!(read(&r.join("original/a.png")), "old");
}

#[test]
fn larger_result_keeps_original() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    let tool = CopyTool {
        grow: true,
        ..png_tool()
    };
    let rep = run_with(
        &tool,
        &spec(vec![r.clone()], false, OutputPolicy::default()),
        &logs,
    );
    assert!(matches!(&rep.files[0], FileResult::Skipped { reason, .. } if reason == "용량 증가"));
    assert!(!r.join("original").exists());
    assert_eq!(read(&r.join("a.png")), "A");
    assert!(no_tmp_dirs(&r));
}

#[test]
fn failing_and_panicking_files_keep_originals() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    for n in ["a", "b", "c"] {
        write(&r, &format!("{n}.png"), n);
    }
    let tool = CopyTool {
        fail_on: Some("a"),
        panic_on: Some("b"),
        ..png_tool()
    };
    let rep = run_with(
        &tool,
        &spec(vec![r.clone()], false, OutputPolicy::default()),
        &logs,
    );
    assert!(matches!(&rep.files[0], FileResult::Failed { .. }));
    assert!(
        matches!(&rep.files[1], FileResult::Failed { message, .. } if message == "도구 실행 중 오류가 발생했습니다.")
    );
    assert!(matches!(&rep.files[2], FileResult::Done { .. }));
    assert_eq!(read(&r.join("a.png")), "a");
    assert_eq!(read(&r.join("b.png")), "b");
    assert!(!r.join("original/a.png").exists());
    assert!(!r.join("original/b.png").exists());
    assert!(no_tmp_dirs(&r));
}

#[test]
fn extension_change_moves_only_original() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    let tool = CopyTool {
        rename_to_ext: Some("webp"),
        ..png_tool()
    };
    run_with(
        &tool,
        &spec(vec![r.clone()], false, OutputPolicy::default()),
        &logs,
    );
    assert_eq!(read(&r.join("a.webp")), "A");
    assert!(!r.join("a.png").exists());
    assert_eq!(read(&r.join("original/a.png")), "A");
}

#[test]
fn in_place_output_name_collision_gets_number() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    write(&r, "a.webp", "keep");
    let tool = CopyTool {
        rename_to_ext: Some("webp"),
        ..png_tool()
    };
    run_with(
        &tool,
        &spec(vec![r.join("a.png")], false, OutputPolicy::default()),
        &logs,
    );
    assert_eq!(read(&r.join("a.webp")), "keep");
    assert_eq!(read(&r.join("a (1).webp")), "A");
}

#[test]
fn escaping_output_path_fails() {
    let d = tempfile::tempdir().unwrap();
    let (r, logs) = (d.path().join("work"), d.path().join("logs"));
    write(&r, "a.png", "A");
    let tool = CopyTool {
        escape: true,
        ..png_tool()
    };
    let rep = run_with(
        &tool,
        &spec(vec![r.clone()], false, OutputPolicy::default()),
        &logs,
    );
    assert!(
        matches!(&rep.files[0], FileResult::Failed { message, .. } if message == "도구가 잘못된 결과 경로를 돌려주었습니다.")
    );
    assert_eq!(read(&r.join("a.png")), "A");
    assert!(!r.join("original").exists());
}
