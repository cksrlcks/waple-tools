use std::path::{Path, PathBuf};

use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

/// 글리프 3개짜리 최소 TrueType (tests/fixtures/mini.ttf)
fn mini_ttf() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/mini.ttf"))
        .unwrap()
}

/// `name` 파일로 두고 변환한 (결과 이름, 내용)
fn run(name: &str, bytes: &[u8]) -> Result<(PathBuf, Vec<u8>), ToolError> {
    run_to(name, bytes, json!({}))
}

fn run_to(
    name: &str,
    bytes: &[u8],
    opts: serde_json::Value,
) -> Result<(PathBuf, Vec<u8>), ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join(name);
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, std::slice::from_ref(&src), &out, &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    let data = std::fs::read(out.join(&names[0])).unwrap();
    Ok((names[0].clone(), data))
}

/// sfnt의 maxp.numGlyphs
fn num_glyphs(font: &[u8]) -> u16 {
    let n = u16::from_be_bytes([font[4], font[5]]) as usize;
    let rec = (0..n)
        .map(|i| &font[12 + 16 * i..28 + 16 * i])
        .find(|r| &r[..4] == b"maxp")
        .expect("maxp");
    let off = u32::from_be_bytes(rec[8..12].try_into().unwrap()) as usize;
    u16::from_be_bytes([font[off + 4], font[off + 5]])
}

#[test]
fn ttf_to_woff2_and_back() {
    let ttf = mini_ttf();
    let (name, woff2) = run("mini.ttf", &ttf).unwrap();
    assert_eq!(name, PathBuf::from("mini.woff2"));
    assert_eq!(&woff2[..4], b"wOF2");

    let (name, back) = run_to("mini.woff2", &woff2, json!({ "format": "sfnt" })).unwrap();
    assert_eq!(name, PathBuf::from("mini.ttf"));
    assert_eq!(&back[..4], &[0, 1, 0, 0]);
    assert_eq!(num_glyphs(&back), 3);
}

#[test]
fn format_comes_from_content_not_extension() {
    // 확장자가 .woff2여도 내용이 TTF면 WOFF2로 압축한다
    let (name, out) = run("odd.woff2", &mini_ttf()).unwrap();
    assert_eq!(name, PathBuf::from("odd.woff2"));
    assert_eq!(&out[..4], b"wOF2");
}

#[test]
fn bad_files_are_korean_errors() {
    let e = run("x.ttf", b"hello world").unwrap_err();
    assert!(
        e.to_string().contains("폰트 파일을 읽을 수 없습니다"),
        "{e}"
    );
    let e = run("x.ttc", b"ttcf\0\0\0\0").unwrap_err();
    assert!(e.to_string().contains("TTC"), "{e}");
    let e = run_to("x.woff2", b"wOF2 broken", json!({ "format": "sfnt" })).unwrap_err();
    assert!(e.to_string().contains("손상"), "{e}");
}

#[test]
fn already_in_target_format_is_reported() {
    let ttf = mini_ttf();
    let (_, woff2) = run("mini.ttf", &ttf).unwrap();
    let e = run("mini.woff2", &woff2).unwrap_err();
    assert!(e.to_string().contains("이미 WOFF2"), "{e}");
    let e = run_to("mini.ttf", &ttf, json!({ "format": "sfnt" })).unwrap_err();
    assert!(e.to_string().contains("이미 TTF"), "{e}");
}

#[test]
fn schema_allows_larger_results() {
    assert_eq!(tool().options_schema()["x-allow-larger"], true);
    assert_eq!(
        tool().options_schema()["x-per-file"]["format"]["woff2"],
        "sfnt"
    );
}
