use serde_json::json;
use waple_tools_core::{JobContext, ToolOutput};
use waple_tools_pdf_common::testkit::{image_pdf, images, page_labels, sample_pdf};

use super::*;

/// `bytes`를 `doc.pdf`로 두고 압축한 결과
fn compress(bytes: &[u8], opts: serde_json::Value) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool()
        .run(&ctx, std::slice::from_ref(&src), &out, &opts)
        .unwrap()
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(names, [PathBuf::from("doc.pdf")]);
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    std::fs::read(out.join(&names[0])).unwrap()
}

#[test]
fn light_keeps_text_and_shrinks_uncompressed() {
    // sample_pdf는 내용 스트림을 압축하지 않고 저장한다
    let src = sample_pdf(30, "t");
    let out = compress(&src, json!({}));
    assert!(out.len() < src.len(), "{} → {}", src.len(), out.len());
    assert_eq!(page_labels(&out), page_labels(&src));
}

#[test]
fn light_leaves_images_alone() {
    let src = image_pdf(1200, 1700);
    let out = compress(&src, json!({ "level": "light" }));
    assert_eq!(images(&out), [(1200, 1700, "FlateDecode".to_string())]);
}

#[test]
fn strong_downsamples_to_max_dpi_as_jpeg() {
    // A4(8.27×11.69in)를 덮는 2000×3000 사진 ≈ 256dpi → 100dpi면 긴 변 1169px, 비율 유지로 779px
    let src = image_pdf(2000, 3000);
    let out = compress(&src, json!({ "level": "strong", "maxDpi": 100 }));
    let imgs = images(&out);
    assert_eq!(imgs.len(), 1);
    let (w, h, filter) = &imgs[0];
    assert_eq!(filter, "DCTDecode");
    assert!((*h - 1169).abs() <= 1, "{w}×{h}");
    assert!((*w - 779).abs() <= 2, "{w}×{h}");
    assert!(out.len() * 4 < src.len(), "{} → {}", src.len(), out.len());
}

#[test]
fn strong_keeps_small_images_size() {
    // 72dpi 미만 사진은 크기를 바꾸지 않는다
    let src = image_pdf(300, 400);
    let out = compress(&src, json!({ "level": "strong", "maxDpi": 150 }));
    let (w, h, _) = &images(&out)[0];
    assert_eq!((*w, *h), (300, 400));
}

#[test]
fn encrypted_is_korean_error() {
    let mut doc = waple_tools_pdf_common::lopdf::Document::load_mem(&sample_pdf(1, "")).unwrap();
    let id = doc.add_object(waple_tools_pdf_common::lopdf::dictionary! { "Filter" => "Standard" });
    doc.trailer.set("Encrypt", id);
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("enc.pdf");
    doc.save(&src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(e.to_string().contains("암호"), "{e}");
}
