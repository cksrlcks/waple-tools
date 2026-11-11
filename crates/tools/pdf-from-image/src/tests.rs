use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::testkit::{alpha_png, jpeg_with_exif, photo};
use waple_tools_pdf_common::lopdf::{Document, Object};
use waple_tools_pdf_common::testkit::images;

use super::*;

/// 이미지 파일들을 넣은 순서대로 묶은 결과 (결과 이름, 내용)
fn build(
    make: impl FnOnce(&Path) -> Vec<PathBuf>,
    opts: serde_json::Value,
) -> Result<(PathBuf, Vec<u8>), ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let inputs = make(dir.path());
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &inputs, &out, &opts)? else {
        panic!("파일이어야 함");
    };
    assert_eq!(names.len(), 1);
    for p in &inputs {
        assert!(p.is_file(), "원본 그대로");
    }
    Ok((
        names[0].clone(),
        std::fs::read(out.join(&names[0])).unwrap(),
    ))
}

fn media_boxes(bytes: &[u8]) -> Vec<(f32, f32)> {
    let doc = Document::load_mem(bytes).unwrap();
    doc.get_pages()
        .values()
        .map(|id| {
            let b = doc
                .get_dictionary(*id)
                .unwrap()
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o.as_float().unwrap())
                .collect::<Vec<_>>();
            (b[2].round(), b[3].round())
        })
        .collect()
}

#[test]
fn one_page_per_image_in_order_jpeg_kept_as_is() {
    let (name, bytes) = build(
        |d| {
            let wide = d.join("b-wide.jpg");
            photo(300, 200).save(&wide).unwrap();
            let tall = d.join("a-tall.png");
            photo(200, 300).save(&tall).unwrap();
            vec![wide, tall]
        },
        json!({}),
    )
    .unwrap();
    assert_eq!(name, PathBuf::from("b-wide.pdf"));
    // 자동 방향: 가로 이미지는 가로 A4, 세로 이미지는 세로 A4
    assert_eq!(media_boxes(&bytes), [(842.0, 595.0), (595.0, 842.0)]);
    let mut imgs = images(&bytes);
    imgs.sort();
    assert_eq!(
        imgs,
        [
            (200, 300, "FlateDecode".to_string()),
            (300, 200, "DCTDecode".to_string())
        ]
    );
}

#[test]
fn image_size_pages_and_recompress() {
    let (_, bytes) = build(
        |d| {
            let p = d.join("x.png");
            photo(960, 480).save(&p).unwrap();
            vec![p]
        },
        json!({ "pageSize": "image", "recompress": true }),
    )
    .unwrap();
    // 96dpi: 960px → 720pt
    assert_eq!(media_boxes(&bytes), [(720.0, 360.0)]);
    assert_eq!(images(&bytes), [(960, 480, "DCTDecode".to_string())]);
}

#[test]
fn rotated_jpeg_and_transparent_png_are_redrawn() {
    let (_, bytes) = build(
        |d| {
            // EXIF 6(90° 회전): 픽셀을 돌려 넣어야 하므로 그대로 넣지 않는다
            let j = d.join("r.jpg");
            jpeg_with_exif(&j, 300, 200, 6, false);
            let a = d.join("a.png");
            alpha_png(&a, 50, 40);
            vec![j, a]
        },
        json!({ "pageSize": "letter" }),
    )
    .unwrap();
    let imgs = images(&bytes);
    assert!(
        imgs.contains(&(200, 300, "FlateDecode".to_string())),
        "{imgs:?}"
    );
    assert!(
        imgs.contains(&(50, 40, "FlateDecode".to_string())),
        "{imgs:?}"
    );
    // Letter 세로(돌린 뒤 세로 이미지) + 가로
    assert_eq!(media_boxes(&bytes), [(612.0, 792.0), (792.0, 612.0)]);
    let doc = Document::load_mem(&bytes).unwrap();
    assert!(
        doc.objects
            .values()
            .all(|o| !matches!(o, Object::Stream(s) if s.dict.has(b"SMask")))
    );
}

#[test]
fn broken_image_names_the_file() {
    let e = build(
        |d| {
            let p = d.join("bad.png");
            std::fs::write(&p, "nope").unwrap();
            vec![p]
        },
        json!({}),
    )
    .unwrap_err();
    assert!(e.to_string().starts_with("bad.png: "), "{e}");
}
