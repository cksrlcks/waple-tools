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
