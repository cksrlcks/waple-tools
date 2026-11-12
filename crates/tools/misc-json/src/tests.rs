use serde_json::json;
use waple_tools_core::{Block, JobContext, ToolOutput};

use super::*;

fn run(opts: serde_json::Value) -> Vec<Block> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    match tool().run(&ctx, &[], out.path(), &opts).unwrap() {
        ToolOutput::Blocks(b) => {
            assert_eq!(
                std::fs::read_dir(out.path()).unwrap().count(),
                0,
                "파일을 만들지 않는다"
            );
            b
        }
        ToolOutput::Files(_) => panic!("블록이어야 함"),
    }
}

/// 성공하면 결과 JSON 코드
fn fmt(text: &str, indent: &str, sort: bool) -> String {
    let blocks = run(json!({ "text": text, "indent": indent, "sortKeys": sort }));
    match &blocks[0] {
        Block::Code { lang, code, name } => {
            assert_eq!(lang, "json");
            assert_eq!(name.as_deref(), Some("formatted.json"));
            code.clone()
        }
        other => panic!(
            "코드 블록이어야 함: {}",
            serde_json::to_string(other).unwrap()
        ),
    }
}

/// 실패하면 안내 문구
fn error(text: &str) -> String {
    let blocks = run(json!({ "text": text }));
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        Block::Text { text } => text.clone(),
        _ => panic!("문구여야 함"),
    }
}

const SAMPLE: &str = r#"{"a":1,"b":[true,null,{}],"c":{"d":"x"},"e":[]}"#;

#[test]
fn indent_two_four_tab() {
    assert_eq!(
        fmt(SAMPLE, "two", false),
        "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    null,\n    {}\n  ],\n  \"c\": {\n    \"d\": \"x\"\n  },\n  \"e\": []\n}"
    );
    assert_eq!(
        fmt("[1,[2]]", "four", false),
        "[\n    1,\n    [\n        2\n    ]\n]"
    );
    assert_eq!(
        fmt("{\"a\":[1]}", "tab", false),
        "{\n\t\"a\": [\n\t\t1\n\t]\n}"
    );
}

#[test]
fn minify_removes_whitespace() {
    let pretty = "{\r\n  \"a\" : [ 1 , 2 ],\n\t\"b\" :{ }\n}\n";
    assert_eq!(fmt(pretty, "minify", false), r#"{"a":[1,2],"b":{}}"#);
    // 문자열 안 공백은 그대로
    assert_eq!(fmt(r#"[ " a  b " ]"#, "minify", false), r#"[" a  b "]"#);
}

#[test]
fn defaults_two_spaces_without_sort() {
    let blocks = run(json!({ "text": r#"{"b":1,"a":2}"# }));
    let Block::Code { code, .. } = &blocks[0] else {
        panic!()
    };
    assert_eq!(code, "{\n  \"b\": 1,\n  \"a\": 2\n}");
}

#[test]
fn sort_keys_nested_and_keeps_duplicates() {
    let src = r#"{"b":{"z":1,"y":[{"q":1,"p":2}]},"a":1,"b":2,"A":3}"#;
    assert_eq!(
        fmt(src, "minify", true),
        r#"{"A":3,"a":1,"b":{"y":[{"p":2,"q":1}],"z":1},"b":2}"#
    );
    // 정렬하지 않으면 원래 순서·중복 그대로
    assert_eq!(fmt(src, "minify", false), src);
}

#[test]
fn numbers_and_escapes_unchanged() {
    let src = r#"[12345678901234567890,1.0,1e10,-0,1E+2,0.1e-7,"é\n\"\/","é한글😀"]"#;
    assert_eq!(fmt(src, "minify", false), src);
    let pretty = fmt(src, "two", false);
    assert!(pretty.contains("12345678901234567890,"));
    assert!(pretty.contains("  1.0,"));
    assert!(pretty.contains("  -0,"));
    assert!(pretty.contains(r#""é\n\"\/""#));
}

#[test]
fn sizes_in_bytes() {
    let blocks = run(json!({ "text": "{ \"a\" : 1 }", "indent": "minify" }));
    let nums: Vec<_> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::Number { label, value, .. } => Some((label.as_str(), *value)),
            _ => None,
        })
        .collect();
    assert_eq!(nums, [("원래 크기", 11.0), ("결과 크기", 7.0)]);
}

#[test]
fn bom_stripped() {
    assert_eq!(fmt("\u{feff}[1]", "minify", false), "[1]");
}

#[test]
fn empty_input_guides() {
    for text in ["", "  \n\t", "\u{feff}"] {
        assert_eq!(error(text), "JSON을 붙여넣거나 파일을 끌어다 놓으세요.");
    }
    assert_eq!(
        error_of(json!(null)),
        "JSON을 붙여넣거나 파일을 끌어다 놓으세요."
    );
}

fn error_of(opts: serde_json::Value) -> String {
    match &run(opts)[0] {
        Block::Text { text } => text.clone(),
        _ => panic!("문구여야 함"),
    }
}

#[test]
fn error_line_and_column() {
    let src = "{\n  \"a\": 1,\n  \"b\": 2 \"c\": 3\n}";
    assert_eq!(
        error(src),
        "3번째 줄 10번째 글자: 쉼표(,)나 닫는 괄호(})가 필요합니다."
    );
    // 열은 바이트가 아니라 글자로 센다
    assert_eq!(
        error("[\"한글\" 1]"),
        "1번째 줄 7번째 글자: 쉼표(,)나 닫는 괄호(])가 필요합니다."
    );
    assert!(error("[1,2,]").starts_with("1번째 줄 5번째 글자: 마지막 쉼표"));
    assert!(error("{'a':1}").contains("큰따옴표"));
    assert!(error("{\"a\" 1}").contains("콜론"));
    assert!(error("[1").contains("배열이 닫히지 않았습니다"));
    assert!(error("{\"a\":1").contains("객체가 닫히지 않았습니다"));
    assert!(error("[\"abc").starts_with("1번째 줄 2번째 글자: 문자열이 닫히지"));
    assert!(error("[\"a\nb\"]").contains("줄바꿈"));
    assert!(error(r#"["\x"]"#).contains("이스케이프"));
    assert!(error(r#"["\u12"]"#).contains("16진수"));
    assert!(error("[01]").contains("0으로 시작"));
    assert!(error("[1.]").contains("소수점"));
    assert!(error("[1e]").contains("지수"));
    assert!(error("[-]").contains("숫자 형식"));
    assert!(error("[tru]").contains("값"));
    assert!(error("1 2").contains("값 뒤에 다른 내용"));
    assert!(error("[").contains("JSON이 끝났습니다"));
}

#[test]
fn deep_nesting_limited() {
    let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert_eq!(fmt(&ok, "minify", false), ok);
    let deep = "[".repeat(1_000_000);
    assert!(error(&deep).contains("중첩이 너무 깊습니다"));
    let deep_obj = "{\"a\":".repeat(100_000);
    assert!(error(&deep_obj).contains("중첩이 너무 깊습니다"));
}

#[test]
fn schema_has_no_input_and_multiline_text() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["properties"]["text"]["x-multiline"], true);
    assert_eq!(s["properties"]["sortKeys"]["type"], "boolean");
}
