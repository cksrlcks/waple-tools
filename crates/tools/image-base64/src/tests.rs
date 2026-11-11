use std::path::PathBuf;

use serde_json::json;
use waple_tools_core::{Block, JobContext, ToolError, ToolOutput};

use super::*;

const PIXEL_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures")
        .join(name)
}

fn run(path: PathBuf, opts: serde_json::Value) -> Result<Vec<Block>, ToolError> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    match tool().run(&ctx, &[path], out.path(), &opts)? {
        ToolOutput::Blocks(b) => {
            assert_eq!(
                std::fs::read_dir(out.path()).unwrap().count(),
                0,
                "파일을 만들지 않는다"
            );
            Ok(b)
        }
        ToolOutput::Files(_) => panic!("블록이어야 함"),
    }
}

fn code(blocks: &[Block]) -> (&str, &str) {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Code { lang, code } => Some((lang.as_str(), code.as_str())),
            _ => None,
        })
        .unwrap()
}

#[test]
fn data_uri_by_default() {
    let b = run(fixture("pixel.png"), json!({})).unwrap();
    assert_eq!(
        code(&b),
        (
            "text",
            format!("data:image/png;base64,{PIXEL_B64}").as_str()
        )
    );
    assert!(
        !b.iter().any(|b| matches!(b, Block::Text { .. })),
        "작은 파일은 경고 없음"
    );
}

#[test]
fn css_and_img_tag() {
    let b = run(fixture("pixel.png"), json!({ "format": "css" })).unwrap();
    assert_eq!(code(&b).0, "css");
    assert!(
        code(&b)
            .1
            .starts_with("background-image: url(\"data:image/png;base64,")
    );
    let b = run(fixture("pixel.png"), json!({ "format": "imgTag" })).unwrap();
    assert_eq!(code(&b).0, "html");
    assert!(code(&b).1.starts_with("<img src=\"data:image/png;base64,"));
    assert!(code(&b).1.ends_with("\" alt=\"\">"));
}

#[test]
fn svg_is_detected_by_content() {
    let b = run(fixture("square.svg"), json!({})).unwrap();
    assert!(code(&b).1.starts_with("data:image/svg+xml;base64,"));
}

#[test]
fn mime_ignores_extension() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("actually-png.jpg");
    std::fs::copy(fixture("pixel.png"), &p).unwrap();
    assert!(
        code(&run(p, json!({})).unwrap())
            .1
            .starts_with("data:image/png;")
    );
    let p = dir.path().join("not-image.png");
    std::fs::write(&p, "hello").unwrap();
    assert!(matches!(run(p, json!({})), Err(ToolError::Failed(m)) if m.contains("지원하지 않는")));
}

#[test]
fn large_file_warns_and_huge_file_fails() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("big.png");
    let mut bytes = std::fs::read(fixture("pixel.png")).unwrap();
    bytes.resize(40 * 1024, 0);
    std::fs::write(&p, &bytes).unwrap();
    let b = run(p.clone(), json!({})).unwrap();
    assert!(matches!(&b[0], Block::Text { text } if text.contains("40 KB")));
    bytes.resize(11 * 1024 * 1024, 0);
    std::fs::write(&p, &bytes).unwrap();
    assert!(matches!(run(p, json!({})), Err(ToolError::Failed(m)) if m.contains("10MB")));
}

#[test]
fn bad_option_is_invalid() {
    let r = run(fixture("pixel.png"), json!({ "format": "nope" }));
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
}
