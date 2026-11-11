use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_pdf_common::testkit::sample_pdf;

use super::*;

/// `doc.pdf`(쪽수 `pages`)를 바꿔 (상대 경로, 이미지) 목록을 돌려준다
fn convert(
    pages: u32,
    opts: serde_json::Value,
) -> Result<Vec<(String, image::DynamicImage)>, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(pages, "")).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, std::slice::from_ref(&src), &out, &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert!(src.is_file());
    Ok(names
        .iter()
        .map(|n| {
            (
                n.to_string_lossy().replace('\\', "/"),
                image::open(out.join(n)).unwrap(),
            )
        })
        .collect())
}

#[test]
fn pages_become_jpgs_at_dpi() {
    let r = convert(3, json!({ "dpi": "d72" })).unwrap();
    let names: Vec<_> = r.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["doc/page-001.jpg", "doc/page-002.jpg", "doc/page-003.jpg"]
    );
    let img = r[0].1.to_rgb8();
    assert_eq!(img.dimensions(), (595, 842));
    // 왼쪽 아래 파란 사각형(50~250pt)은 파랗고, 위쪽 빈 곳은 흰색
    let blue = img.get_pixel(150, 842 - 150).0;
    assert!(blue[2] > 150 && blue[0] < 100, "{blue:?}");
    let white = img.get_pixel(500, 50).0;
    assert!(white.iter().all(|&c| c > 240), "{white:?}");
    // 글자(Helvetica 대체 글꼴)가 그려졌다: 글자 줄에 어두운 화소가 있다
    let dark = (100..400)
        .flat_map(|x| (230..250).map(move |y| (x, y)))
        .any(|(x, y)| img.get_pixel(x, y).0[0] < 100);
    assert!(dark, "글자가 그려져야 함");
}

#[test]
fn png_custom_dpi_and_range() {
    let r = convert(
        5,
        json!({ "format": "png", "dpi": "custom", "customDpi": 144, "pages": "2,4-" }),
    )
    .unwrap();
    let names: Vec<_> = r.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["doc/page-002.png", "doc/page-004.png", "doc/page-005.png"]
    );
    assert_eq!((r[0].1.width(), r[0].1.height()), (1190, 1684));
}

#[test]
fn bad_range_and_garbage_are_korean_errors() {
    let e = convert(2, json!({ "pages": "3" })).unwrap_err();
    assert!(e.to_string().contains("페이지 수(2)"), "{e}");

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("bad.pdf");
    std::fs::write(&src, "%PDF-1.4 broken").unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(e.to_string().contains("PDF를 읽을 수 없습니다"), "{e}");
}

#[test]
fn thumbnails_for_every_page() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(3, "")).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    // 페이지 범위 옵션과 상관없이 모든 쪽
    let opts = json!({ "x-thumbnails": true, "pages": "2" });
    let ToolOutput::Files(names) = tool().run(&ctx, &[src], dir.path(), &opts).unwrap() else {
        panic!()
    };
    let names: Vec<_> = names
        .iter()
        .map(|n| n.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        names,
        [
            "thumbs/p-0001.jpg",
            "thumbs/p-0002.jpg",
            "thumbs/p-0003.jpg"
        ]
    );
    let img = image::open(dir.path().join("thumbs/p-0001.jpg")).unwrap();
    assert_eq!(img.height(), 240, "긴 변 240px");
}
