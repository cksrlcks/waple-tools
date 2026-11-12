use serde_json::json;
use waple_tools_core::{Block, DiffLayout, DiffLine, DiffTag, JobContext, Span, ToolOutput};

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

fn diff(a: &str, b: &str) -> Vec<DiffLine> {
    diff_with(json!({ "original": a, "modified": b }))
}

fn diff_with(opts: serde_json::Value) -> Vec<DiffLine> {
    run(opts)
        .into_iter()
        .find_map(|b| match b {
            Block::Diff { lines, .. } => Some(lines),
            _ => None,
        })
        .expect("diff 블록")
}

fn text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// 표시된 글자를 [ ]로 감싼 줄
fn marked(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| match s.mark {
            Some(0) => format!("[{}]", s.text),
            None => s.text.clone(),
            Some(_) => panic!("mark는 0만"),
        })
        .collect()
}

/// (태그, 원본 번호, 비교 번호, 글)
fn summary(lines: &[DiffLine]) -> Vec<(DiffTag, Option<u32>, Option<u32>, String)> {
    lines
        .iter()
        .map(|l| (l.tag, l.old, l.new, text(&l.spans)))
        .collect()
}

fn number(blocks: &[Block], label: &str) -> f64 {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Number {
                label: l, value, ..
            } if l == label => Some(*value),
            _ => None,
        })
        .unwrap()
}

fn only_text(blocks: &[Block]) -> &str {
    match blocks {
        [Block::Text { text }] => text,
        other => panic!("안내 글 하나여야 함: {other:?}"),
    }
}

use DiffTag::{Delete, Equal, Insert};

#[test]
fn insert_delete_replace_with_line_numbers() {
    let lines = diff("a\nb\nc\nd", "a\nc\nX\nd\ne");
    assert_eq!(
        summary(&lines),
        vec![
            (Equal, Some(1), Some(1), "a".into()),
            (Delete, Some(2), None, "b".into()),
            (Equal, Some(3), Some(2), "c".into()),
            (Insert, None, Some(3), "X".into()),
            (Equal, Some(4), Some(4), "d".into()),
            (Insert, None, Some(5), "e".into()),
        ]
    );
    let b = run(json!({ "original": "a\nb\nc\nd", "modified": "a\nc\nX\nd\ne" }));
    assert_eq!(number(&b, "추가된 줄"), 2.0);
    assert_eq!(number(&b, "삭제된 줄"), 1.0);
}

#[test]
fn replaced_lines_come_deletes_then_inserts_with_inline_marks() {
    let lines = diff(
        "x\nhello world\nfoo bar\ny",
        "x\nhello there world\nfoo baz\ny",
    );
    assert_eq!(
        lines.iter().map(|l| l.tag).collect::<Vec<_>>(),
        [Equal, Delete, Delete, Insert, Insert, Equal]
    );
    assert_eq!(marked(&lines[1].spans), "hello world");
    assert_eq!(marked(&lines[3].spans), "hello [there ]world");
    assert_eq!(marked(&lines[2].spans), "foo ba[r]");
    assert_eq!(marked(&lines[4].spans), "foo ba[z]");
    assert!(lines[0].spans.iter().all(|s| s.mark.is_none()));
}

#[test]
fn totally_different_lines_have_no_inline_marks() {
    let lines = diff("abcdef", "uvwxyz");
    assert_eq!(lines.len(), 2);
    assert!(
        lines
            .iter()
            .all(|l| l.spans.len() == 1 && l.spans[0].mark.is_none())
    );
}

#[test]
fn ignore_whitespace_compares_normalized_but_shows_original() {
    let opts = json!({ "original": "a  b\nfoo", "modified": "a b\nfoo", "ignoreWhitespace": true });
    assert_eq!(
        only_text(&run(opts)),
        "무시한 차이 말고는 두 글이 같습니다."
    );

    let lines = diff_with(json!({
        "original": "if (x)  {\nkeep",
        "modified": "if(x){\nkeep\n  added  ",
        "ignoreWhitespace": true,
    }));
    assert_eq!(
        summary(&lines),
        vec![
            (Equal, Some(1), Some(1), "if (x)  {".into()),
            (Equal, Some(2), Some(2), "keep".into()),
            (Insert, None, Some(3), "  added  ".into()),
        ]
    );
    // 꺼져 있으면 다른 줄
    assert_eq!(diff("a  b", "a b").len(), 2);
}

#[test]
fn ignore_whitespace_never_marks_whitespace_inline() {
    let lines = diff_with(json!({
        "original": "let  a = 1;",
        "modified": "let a = 2;",
        "ignoreWhitespace": true,
    }));
    assert_eq!(marked(&lines[0].spans), "let  a = [1];");
    assert_eq!(marked(&lines[1].spans), "let a = [2];");
}

#[test]
fn ignore_case_shows_original_text() {
    let lines = diff_with(json!({
        "original": "Hello\nWORLD one",
        "modified": "hello\nworld two",
        "ignoreCase": true,
    }));
    assert_eq!(
        summary(&lines),
        vec![
            (Equal, Some(1), Some(1), "Hello".into()),
            (Delete, Some(2), None, "WORLD one".into()),
            (Insert, None, Some(2), "world two".into()),
        ]
    );
    // 대소문자만 다른 글자는 표시하지 않는다
    assert_eq!(marked(&lines[1].spans), "WORLD o[ne]");
    assert_eq!(marked(&lines[2].spans), "world [tw]o");
}

#[test]
fn crlf_and_lf_are_the_same() {
    let b = run(json!({ "original": "a\r\nb\r\n", "modified": "a\nb\n" }));
    assert_eq!(only_text(&b), "두 글이 같습니다.");
    let lines = diff("a\r\nb\r\nc", "a\nB\nc\n");
    assert_eq!(
        summary(&lines),
        vec![
            (Equal, Some(1), Some(1), "a".into()),
            (Delete, Some(2), None, "b".into()),
            (Insert, None, Some(2), "B".into()),
            (Equal, Some(3), Some(3), "c".into()),
        ]
    );
    assert!(lines.iter().all(|l| !text(&l.spans).contains('\r')));
}

#[test]
fn identical_and_empty() {
    assert_eq!(
        only_text(&run(
            json!({ "original": "같은\n글", "modified": "같은\n글" })
        )),
        "두 글이 같습니다."
    );
    for opts in [
        json!({}),
        json!(null),
        json!({ "original": "", "modified": "" }),
    ] {
        assert!(only_text(&run(opts)).contains("붙여 넣거나"));
    }
    // 한쪽만 비면 전부 추가
    let lines = diff("", "a\nb");
    assert_eq!(
        summary(&lines),
        vec![
            (Insert, None, Some(1), "a".into()),
            (Insert, None, Some(2), "b".into()),
        ]
    );
}

#[test]
fn korean_marks_whole_characters() {
    let lines = diff("안녕하세요 반갑습니다", "안녕하십니까 반갑습니다");
    assert_eq!(marked(&lines[0].spans), "안녕하[세요] 반갑습니다");
    assert_eq!(marked(&lines[1].spans), "안녕하[십니까] 반갑습니다");
    let lines = diff("가나다", "가너다");
    assert_eq!(marked(&lines[0].spans), "가[나]다");
    assert_eq!(marked(&lines[1].spans), "가[너]다");
}

#[test]
fn layout_option_is_passed_through() {
    let layout = |opts| {
        run(opts)
            .into_iter()
            .find_map(|b| match b {
                Block::Diff { layout, .. } => Some(layout),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        layout(json!({ "original": "a", "modified": "b" })),
        DiffLayout::Split
    );
    assert_eq!(
        layout(json!({ "original": "a", "modified": "b", "layout": "unified" })),
        DiffLayout::Unified
    );
}

#[test]
fn big_input_is_capped_and_fast() {
    let a: String = (0..60_000).map(|i| format!("line {i}\n")).collect();
    let b: String = (0..60_000).map(|i| format!("row {}\n", i * 7)).collect();
    let started = Instant::now();
    let blocks = run(json!({ "original": a, "modified": b }));
    assert!(started.elapsed() < Duration::from_secs(10), "너무 느림");
    assert!(matches!(&blocks[0], Block::Text { text } if text.contains("너무 길어")));
    assert_eq!(number(&blocks, "삭제된 줄"), 60_000.0);
    assert_eq!(number(&blocks, "추가된 줄"), 60_000.0);
    assert!(diff_with(json!({ "original": a, "modified": b })).len() <= MAX_LINES);
}

#[test]
fn schema_marks_text_inputs() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    for k in ["original", "modified"] {
        assert_eq!(s["properties"][k]["x-multiline"], true);
        assert_eq!(s["properties"][k]["type"], "string");
    }
    assert_eq!(s["properties"]["ignoreWhitespace"]["type"], "boolean");
    assert_eq!(s["properties"]["ignoreCase"]["type"], "boolean");
}
