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

fn count(b: &[Block]) -> f64 {
    b.iter()
        .find_map(|b| match b {
            Block::Number { label, value, .. } if label == "일치 수" => Some(*value),
            _ => None,
        })
        .unwrap()
}

fn texts(b: &[Block]) -> Vec<&str> {
    b.iter()
        .filter_map(|b| match b {
            Block::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn spans(b: &[Block]) -> &[Span] {
    b.iter()
        .find_map(|b| match b {
            Block::Highlight { spans } => Some(spans.as_slice()),
            _ => None,
        })
        .unwrap()
}

fn records(b: &[Block]) -> &[Record] {
    b.iter()
        .find_map(|b| match b {
            Block::Records { items } => Some(items.as_slice()),
            _ => None,
        })
        .unwrap()
}

/// (위치, 일치, 그룹 값들)
fn rows(b: &[Block]) -> Vec<(String, String, Vec<String>)> {
    records(b)
        .iter()
        .map(|r| {
            (
                r.meta.clone().unwrap(),
                r.text.clone(),
                r.fields.iter().map(|f| f.value.clone()).collect(),
            )
        })
        .collect()
}

fn joined(s: &[Span]) -> String {
    s.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn schema_marks_inputs() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["x-layout"], "split");
    assert_eq!(s["properties"]["pattern"]["format"], "regex");
    assert_eq!(s["properties"]["text"]["x-multiline"], true);
    assert_eq!(s["properties"]["ignoreCase"]["x-short"], "i");
    assert_eq!(s["properties"]["global"]["x-short"], "g");
}

#[test]
fn guidance_when_empty() {
    let b = run(json!({}));
    assert_eq!(texts(&b), ["정규식을 입력해 주세요."]);
    let b = run(json!({ "pattern": "a" }));
    assert!(texts(&b)[0].contains("테스트할 글"));
    // 글이 없어도 패턴 오류는 먼저 알려 준다
    let b = run(json!({ "pattern": "(" }));
    assert!(texts(&b)[0].contains("올바르지 않습니다"));
}

#[test]
fn korean_text_positions_in_chars() {
    let t = "가나다 abc\n라마 abc";
    let b = run(json!({ "pattern": "abc", "text": t }));
    assert_eq!(count(&b), 2.0);
    let s = spans(&b);
    assert_eq!(joined(s), t);
    let marked: Vec<_> = s
        .iter()
        .filter_map(|s| s.mark.map(|m| (m, s.text.as_str())))
        .collect();
    assert_eq!(marked, [(0, "abc"), (1, "abc")]);
    let r = records(&b);
    assert_eq!(r[0].title, "매치 1");
    assert_eq!(r[0].meta.as_deref(), Some("위치 4–7"));
    assert_eq!(r[1].meta.as_deref(), Some("위치 11–14"));
    assert!(r[0].fields.is_empty());
}

#[test]
fn multiline_flag() {
    let t = "하나\n둘\n셋";
    let b = run(json!({ "pattern": "^.+$", "text": t }));
    assert_eq!(count(&b), 0.0);
    let b = run(json!({ "pattern": "^.+$", "multiLine": true, "text": t }));
    assert_eq!(count(&b), 3.0);
    assert_eq!(rows(&b)[2].0, "위치 5–6");
    assert_eq!(rows(&b)[2].1, "셋");
}

#[test]
fn ignore_case_and_dot_all() {
    let b = run(json!({ "pattern": "abc", "ignoreCase": true, "text": "ABC abc" }));
    assert_eq!(count(&b), 2.0);
    let b = run(json!({ "pattern": "a.b", "text": "a\nb" }));
    assert_eq!(count(&b), 0.0);
    let b = run(json!({ "pattern": "a.b", "dotAll": true, "text": "a\nb" }));
    assert_eq!(count(&b), 1.0);
}

#[test]
fn groups_named_unnamed_and_unmatched() {
    let b = run(json!({
        "pattern": r"(?<year>\d{4})-(\d{2})(-x)?",
        "text": "2024-05 2025-11-x"
    }));
    let labels: Vec<_> = records(&b)[0]
        .fields
        .iter()
        .map(|f| f.label.as_str())
        .collect();
    assert_eq!(labels, ["year", "그룹 2", "그룹 3"]);
    let r = rows(&b);
    assert_eq!(
        r[0],
        (
            "위치 0–7".into(),
            "2024-05".into(),
            vec!["2024".into(), "05".into(), String::new()]
        )
    );
    assert_eq!(
        r[1],
        (
            "위치 8–17".into(),
            "2025-11-x".into(),
            vec!["2025".into(), "11".into(), "-x".into()]
        )
    );
}

#[test]
fn js_features_lookaround_and_backref() {
    let b = run(json!({ "pattern": r"\d+(?=원)", "text": "100원 200달러" }));
    assert_eq!(rows(&b)[0].1, "100");
    let b = run(json!({ "pattern": r"(\w)\1", "text": "abba" }));
    assert_eq!(rows(&b)[0].1, "bb");
}

#[test]
fn zero_width_matches_terminate() {
    let t = "가a";
    let b = run(json!({ "pattern": "x*", "text": t }));
    // 각 글자 경계마다 빈 일치: 0, 1, 2 글자 뒤
    assert_eq!(count(&b), 3.0);
    let s = spans(&b);
    assert_eq!(joined(s), t);
    let pos: Vec<_> = rows(&b).into_iter().map(|r| r.0).collect();
    assert_eq!(pos, ["위치 0–0", "위치 1–1", "위치 2–2"]);
    assert!(
        texts(&b)
            .iter()
            .any(|t| t.contains("길이가 0인 일치가 3개"))
    );
}

#[test]
fn invalid_pattern_reports_position() {
    let b = run(json!({ "pattern": "가나(", "text": "x" }));
    assert_eq!(b.len(), 1);
    let msg = texts(&b)[0];
    assert!(msg.contains("정규식이 올바르지 않습니다"), "{msg}");
    assert!(msg.contains("괄호"), "{msg}");
    let b = run(json!({ "pattern": r"a\", "text": "x" }));
    assert!(texts(&b)[0].contains("2번째 글자"), "{}", texts(&b)[0]);
}

#[test]
fn global_off_finds_first_only() {
    let b = run(json!({ "pattern": "a", "global": false, "text": "aaa" }));
    assert_eq!(count(&b), 1.0);
    assert_eq!(records(&b).len(), 1);
}

#[test]
fn caps_shown_matches() {
    let t = "a".repeat(MAX_SHOWN + 5);
    let b = run(json!({ "pattern": "a", "text": t }));
    assert_eq!(count(&b), (MAX_SHOWN + 5) as f64);
    assert!(texts(&b).iter().any(|t| t.contains("앞의 1000개만")));
    let s = spans(&b);
    assert_eq!(joined(s), t);
    assert_eq!(s.iter().filter(|s| s.mark.is_some()).count(), MAX_SHOWN);
    assert_eq!(records(&b).len(), MAX_SHOWN);
}

#[test]
fn catastrophic_pattern_stops_without_panic() {
    let t = format!("{}!", "a".repeat(40));
    let b = run(json!({ "pattern": r"(a+)+\1b", "text": t }));
    assert!(texts(&b)[0].contains("너무 복잡해서"), "{:?}", texts(&b));
}

#[test]
fn no_match() {
    let b = run(json!({ "pattern": "z", "text": "abc" }));
    assert_eq!(count(&b), 0.0);
    assert_eq!(texts(&b), ["일치하는 곳이 없습니다."]);
}
