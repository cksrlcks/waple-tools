use waple_tools_core::ToolError;

use super::testkit::{page_count, page_labels, sample_pdf};
use super::*;

fn err(r: Result<impl std::fmt::Debug, ToolError>) -> String {
    r.unwrap_err().to_string()
}

#[test]
fn ranges_parse() {
    assert_eq!(parse_ranges("", 9).unwrap(), [(1, 9)]);
    assert_eq!(
        parse_ranges(" 1-3, 5 ,8- ", 9).unwrap(),
        [(1, 3), (5, 5), (8, 9)]
    );
    assert_eq!(parse_ranges("-2", 9).unwrap(), [(1, 2)]);
    assert!(err(parse_ranges("3-1", 9)).contains("올바르지 않습니다"));
    assert!(err(parse_ranges("0", 9)).contains("올바르지 않습니다"));
    assert!(err(parse_ranges("10", 9)).contains("페이지 수(9)"));
    assert!(err(parse_ranges("a-b", 9)).contains("읽을 수 없습니다"));
    assert!(err(parse_ranges(",", 9)).contains("입력하세요"));
}

#[test]
fn pages_in_keeps_order_without_duplicates() {
    assert_eq!(pages_in(&[(3, 4), (1, 3)]), [3, 4, 1, 2]);
}

#[test]
fn open_rejects_garbage_and_encrypted() {
    assert!(err(open_bytes(b"hello")).contains("읽을 수 없습니다"));
    assert!(err(open_bytes(b"%PDF-1.4\n garbage")).contains("읽을 수 없습니다"));
    // 트레일러에 /Encrypt를 넣어 저장
    let mut doc = lopdf::Document::load_mem(&sample_pdf(1, "")).unwrap();
    let id = doc.add_object(lopdf::dictionary! { "Filter" => "Standard", "V" => 1 });
    doc.trailer.set("Encrypt", id);
    let mut enc = vec![];
    doc.save_to(&mut enc).unwrap();
    assert!(err(open_bytes(&enc)).contains("암호"));
}

#[test]
fn open_and_save_roundtrip_keeps_pages() {
    let d = tempfile::tempdir().unwrap();
    let mut doc = open_bytes(&sample_pdf(3, "x")).unwrap();
    push_down_inherited(&mut doc);
    let p = d.path().join("out.pdf");
    save(&mut doc, &p).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    assert_eq!(page_count(&bytes), 3);
    assert_eq!(page_labels(&bytes), ["x Page 1", "x Page 2", "x Page 3"]);
    // 물려받던 MediaBox가 페이지에 직접 있다
    let doc = lopdf::Document::load_mem(&bytes).unwrap();
    let first = *doc.get_pages().values().next().unwrap();
    assert!(doc.get_dictionary(first).unwrap().has(b"MediaBox"));
}
