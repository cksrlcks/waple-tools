use serde_json::json;
use waple_tools_core::{JobContext, ToolOutput};
use waple_tools_pdf_common::lopdf::Document;
use waple_tools_pdf_common::testkit::{page_labels, sample_pdf};

use super::*;

/// `(이름, 쪽수)` 파일들을 넣은 순서대로 합친 결과 (결과 이름, 내용)
fn merge_files(files: &[(&str, u32)], opts: serde_json::Value) -> (PathBuf, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    let inputs: Vec<PathBuf> = files
        .iter()
        .map(|(n, pages)| {
            let p = dir.path().join(format!("{n}.pdf"));
            std::fs::write(&p, sample_pdf(*pages, n)).unwrap();
            p
        })
        .collect();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &inputs, &out, &opts).unwrap() else {
        panic!("파일이어야 함");
    };
    assert_eq!(names.len(), 1);
    for p in &inputs {
        assert!(p.is_file(), "원본 그대로");
    }
    let bytes = std::fs::read(out.join(&names[0])).unwrap();
    (names[0].clone(), bytes)
}

#[test]
fn pages_follow_input_order() {
    let (name, bytes) = merge_files(&[("b", 2), ("a", 1), ("c", 2)], json!({}));
    assert_eq!(name, PathBuf::from("b-merged.pdf"));
    assert_eq!(
        page_labels(&bytes),
        ["b Page 1", "b Page 2", "a Page 1", "c Page 1", "c Page 2"]
    );
    // 물려받던 크기·글꼴이 페이지마다 남아 있다
    let doc = Document::load_mem(&bytes).unwrap();
    for id in doc.get_pages().values() {
        let page = doc.get_dictionary(*id).unwrap();
        assert!(page.has(b"MediaBox") && page.has(b"Resources"));
    }
}

#[test]
fn bookmarks_per_file() {
    let (_, bytes) = merge_files(&[("first", 1), ("second", 2)], json!({}));
    let doc = Document::load_mem(&bytes).unwrap();
    let outlines = doc.catalog().unwrap().get(b"Outlines").unwrap();
    let outlines = doc
        .get_dictionary(outlines.as_reference().unwrap())
        .unwrap();
    assert_eq!(outlines.get(b"Count").unwrap().as_i64().unwrap(), 2);

    let (_, bytes) = merge_files(
        &[("first", 1), ("second", 2)],
        json!({ "bookmarks": false }),
    );
    let doc = Document::load_mem(&bytes).unwrap();
    assert!(!doc.catalog().unwrap().has(b"Outlines"));
}

#[test]
fn needs_two_files_and_names_the_broken_one() {
    let dir = tempfile::tempdir().unwrap();
    let ok = dir.path().join("ok.pdf");
    std::fs::write(&ok, sample_pdf(1, "")).unwrap();
    let bad = dir.path().join("bad.pdf");
    std::fs::write(&bad, "nope").unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let e = tool()
        .run(&ctx, std::slice::from_ref(&ok), dir.path(), &json!({}))
        .unwrap_err();
    assert!(e.to_string().contains("두 개 이상"), "{e}");
    let e = tool()
        .run(&ctx, &[ok, bad], dir.path(), &json!({}))
        .unwrap_err();
    assert!(
        e.to_string().starts_with("bad.pdf: PDF를 읽을 수 없습니다"),
        "{e}"
    );
}
