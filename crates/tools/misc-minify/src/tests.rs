use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/misc-minify")
            .join(name),
    )
    .unwrap()
}

/// 임시 폴더에 `name`으로 두고 압축한 결과 (글)
fn run(name: &str, bytes: &[u8], opts: Value) -> Result<String, ToolError> {
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
    assert_eq!(names, vec![PathBuf::from(name)], "입력과 같은 이름");
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    Ok(String::from_utf8(std::fs::read(out.join(name)).unwrap()).unwrap())
}

fn run_fixture(name: &str, opts: Value) -> String {
    let src = fixture(name);
    let out = run(name, &src, opts).unwrap();
    assert!(out.len() < src.len(), "{name}: 결과가 더 작아야 함\n{out}");
    out
}

fn assert_js_parses(code: &str, source_type: SourceType) {
    let alloc = Allocator::default();
    let ret = Parser::new(&alloc, code, source_type).parse();
    assert!(
        ret.errors.is_empty(),
        "다시 읽혀야 함: {:?}\n{code}",
        ret.errors
    );
}

#[test]
fn css_is_smaller_and_still_parses() {
    let out = run_fixture("style.css", json!({}));
    assert!(
        StyleSheet::parse(&out, ParserOptions::default()).is_ok(),
        "{out}"
    );
    assert!(
        out.contains("color:#fff;background-color:red;margin:0;padding:10px 20px"),
        "값을 줄임: {out}"
    );
    assert!(!out.contains("버튼 기본 모양"), "일반 주석은 지움: {out}");
}

#[test]
fn css_license_comment_follows_option() {
    let kept = run_fixture("style.css", json!({}));
    assert!(
        kept.starts_with("/*! sample.css v1.0 | MIT License */"),
        "{kept}"
    );
    let removed = run_fixture("style.css", json!({ "keepLicense": false }));
    assert!(!removed.contains("MIT License"), "{removed}");
}

#[test]
fn js_is_smaller_and_still_parses() {
    let out = run_fixture("app.js", json!({}));
    assert_js_parses(&out, SourceType::cjs());
    assert!(!out.contains("장바구니"), "일반 주석은 지움: {out}");
    // 전역 함수 이름은 다른 스크립트가 부를 수 있어 그대로
    assert!(out.contains("function calculateTotal("), "{out}");
    assert!(out.contains("function render("), "{out}");
}

#[test]
fn js_license_comment_follows_option() {
    let kept = run_fixture("app.js", json!({}));
    assert!(
        kept.contains("/*! app.js v1.0 | (c) 2026 sample | MIT License */"),
        "{kept}"
    );
    let removed = run_fixture("app.js", json!({ "keepLicense": false }));
    assert!(!removed.contains("MIT License"), "{removed}");
    // @license·@preserve 표시도 라이선스 주석
    let src = "/* @license Foo */\nfunction a(){return 1}\n/** @preserve Bar */\nfunction b(){return 2}\n";
    let kept = run("x.js", src.as_bytes(), json!({})).unwrap();
    assert!(
        kept.contains("@license Foo") && kept.contains("@preserve Bar"),
        "{kept}"
    );
}

#[test]
fn js_mangle_follows_option() {
    let on = run_fixture("app.js", json!({}));
    assert!(!on.contains("shoppingCartItems"), "지역 이름을 줄임: {on}");
    let off = run_fixture("app.js", json!({ "mangle": false }));
    assert!(off.contains("shoppingCartItems"), "{off}");
    assert!(on.len() < off.len());
}

#[test]
fn mjs_is_a_module() {
    let out = run_fixture("module.mjs", json!({}));
    assert_js_parses(&out, SourceType::mjs());
    assert!(out.contains("node:fs/promises"), "{out}");
    assert!(
        out.contains("loadConfiguration"),
        "내보낸 이름은 그대로: {out}"
    );
    assert!(out.contains("await "), "{out}");
    // .js에 import·export가 있어도 모듈로 읽는다
    let js = run("module.js", &fixture("module.mjs"), json!({})).unwrap();
    assert_js_parses(&js, SourceType::mjs());
    // .cjs는 스크립트: import는 구문 오류
    let e = run("module.cjs", &fixture("module.mjs"), json!({})).unwrap_err();
    assert!(e.to_string().starts_with("구문 오류("), "{e}");
}

#[test]
fn html_minifies_inline_css_and_js() {
    let out = run_fixture("page.html", json!({}));
    assert!(
        !out.contains("페이지 전용 스타일"),
        "HTML 주석은 지움: {out}"
    );
    assert!(out.contains("안녕하세요"), "{out}");
    assert!(!out.contains("#ffffff"), "<style> 압축: {out}");
    assert!(!out.contains("#00ff00"), "style 속성 압축: {out}");
    // <script>는 minify-html이 oxc로 줄인다: 식은 간단히 하지만 문장 앞 주석·지역 이름은 남긴다
    assert!(!out.contains("greetingMessage"), "<script> 압축: {out}");
    assert!(out.contains("반갑습니다"), "{out}");

    let off = run_fixture("page.html", json!({ "minifyInline": false }));
    assert!(off.contains("#ffffff") && off.contains("#00ff00"), "{off}");
    assert!(off.contains("greetingMessage"), "{off}");
    assert!(out.len() < off.len());
}

#[test]
fn syntax_errors_are_korean_with_position() {
    let e = run("broken.js", &fixture("broken.js"), json!({})).unwrap_err();
    assert!(e.to_string().starts_with("구문 오류(5줄 "), "{e}");
    let e = run("broken.css", &fixture("broken.css"), json!({})).unwrap_err();
    assert!(e.to_string().starts_with("구문 오류(5줄 "), "{e}");
}

#[test]
fn line_col_counts_characters() {
    let src = "가나\n다라마";
    assert_eq!(line_col(src, 0), (1, 1));
    assert_eq!(line_col(src, "가나\n다".len()), (2, 2));
    assert_eq!(line_col(src, 1), (1, 1), "글자 중간은 그 글자 앞");
}

#[test]
fn non_utf8_is_a_korean_error() {
    // EUC-KR "가"
    let e = run("x.css", b"a{content:\"\xb0\xa1\"}", json!({})).unwrap_err();
    assert!(e.to_string().contains("UTF-8"), "{e}");
}

#[test]
fn utf8_bom_is_dropped() {
    let out = run("x.css", "\u{feff}a { color: red; }".as_bytes(), json!({})).unwrap();
    assert_eq!(out, "a{color:red}");
}

#[test]
fn options_default_on() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "none");
    for k in ["mangle", "keepLicense", "minifyInline"] {
        assert_eq!(s["properties"][k]["default"], true, "{k}");
    }
}
