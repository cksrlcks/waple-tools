use std::collections::BTreeMap;

use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_pdf_common::testkit::{page_labels, sample_pdf};

use super::*;

/// `doc.pdf`(5쪽)를 나눠 (이름 → 페이지 글자) 를 돌려준다. 모든 경로는 `doc-split/` 아래
fn split(opts: serde_json::Value) -> Result<BTreeMap<String, Vec<String>>, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(5, "")).unwrap();
    let before = std::fs::read(&src).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, std::slice::from_ref(&src), &out, &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(std::fs::read(&src).unwrap(), before, "원본 그대로");
    Ok(names
        .iter()
        .map(|n| {
            let rel = n.strip_prefix("doc-split").expect("doc-split 아래");
            let labels = page_labels(&std::fs::read(out.join(n)).unwrap())
                .into_iter()
                .map(|l| l.replace("Page ", ""))
                .collect();
            (rel.to_string_lossy().into_owned(), labels)
        })
        .collect())
}

fn pages(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn each_page() {
    let r = split(json!({})).unwrap();
    assert_eq!(r.len(), 5);
    assert_eq!(r["doc_p1.pdf"], pages(&["1"]));
    assert_eq!(r["doc_p5.pdf"], pages(&["5"]));
}

#[test]
fn chunks() {
    let r = split(json!({ "mode": "chunk", "chunk": 2 })).unwrap();
    let names: Vec<_> = r.keys().map(String::as_str).collect();
    assert_eq!(names, ["doc_p1-2.pdf", "doc_p3-4.pdf", "doc_p5.pdf"]);
    assert_eq!(r["doc_p3-4.pdf"], pages(&["3", "4"]));
}

#[test]
fn ranges_separate_and_joined() {
    let r = split(json!({ "mode": "ranges", "ranges": "1-2, 4-" })).unwrap();
    assert_eq!(r["doc_p1-2.pdf"], pages(&["1", "2"]));
    assert_eq!(r["doc_p4-5.pdf"], pages(&["4", "5"]));

    let r = split(json!({ "mode": "ranges", "ranges": "1-2,4", "join": true })).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r["doc_p1-2_4.pdf"], pages(&["1", "2", "4"]));
}

#[test]
fn empty_ranges_mean_whole_document() {
    let r = split(json!({ "mode": "ranges", "ranges": "" })).unwrap();
    assert_eq!(r["doc_p1-5.pdf"], pages(&["1", "2", "3", "4", "5"]));
}

#[test]
fn thumbnails_for_page_picker() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(3, "")).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "x-thumbnails": true });
    let ToolOutput::Files(names) = tool().run(&ctx, &[src], dir.path(), &opts).unwrap() else {
        panic!()
    };
    assert_eq!(names.len(), 3);
    assert!(dir.path().join(&names[2]).is_file());
}

#[test]
fn bad_ranges_are_korean_errors() {
    let e = split(json!({ "mode": "ranges", "ranges": "6" })).unwrap_err();
    assert!(e.to_string().contains("페이지 수(5)"), "{e}");
}

#[test]
fn parts_are_smaller_than_whole() {
    // 지운 페이지 내용이 결과에 남지 않는다
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(40, "")).unwrap();
    let whole = std::fs::metadata(&src).unwrap().len();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "mode": "ranges", "ranges": "1" });
    let ToolOutput::Files(names) = tool().run(&ctx, &[src], &out, &opts).unwrap() else {
        panic!()
    };
    let part = std::fs::metadata(out.join(&names[0])).unwrap().len();
    assert!(part * 5 < whole, "{part} vs {whole}");
}

#[test]
fn garbage_is_korean_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("bad.pdf");
    std::fs::write(&src, "not a pdf").unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(e.to_string().contains("PDF를 읽을 수 없습니다"), "{e}");
}
