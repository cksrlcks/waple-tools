use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::testkit::{jpeg_with_exif, photo};
use waple_tools_image_common::{Fmt, Quality, encode, parse_options};

use crate::crop::crop_rect;
use crate::{EditOptions, tool};

fn run(input: &Path, v: Value) -> Result<PathBuf, ToolError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let out = input
        .parent()
        .unwrap()
        .join(format!("out-{}", N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    match tool().run(&ctx, &[input.to_path_buf()], &out, &v)? {
        ToolOutput::Files(f) => Ok(out.join(&f[0])),
        other => panic!("{other:?}"),
    }
}

fn dims(p: &Path) -> (u32, u32) {
    let i = image::open(p).unwrap();
    (i.width(), i.height())
}

fn jpeg(w: u32, h: u32) -> Vec<u8> {
    encode(
        &photo(w, h),
        Fmt::Jpeg,
        Quality {
            quality: 85,
            lossless: false,
        },
    )
    .unwrap()
}

// ---------- 회전 (옛 회전 도구에서 옮김) ----------

#[test]
fn r90_swaps_dimensions() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(40, 20).save(&src).unwrap();
    assert_eq!(
        dims(&run(&src, json!({ "rotate": true, "angle": "r90" })).unwrap()),
        (20, 40)
    );
}

#[test]
fn flip_h_mirrors_pixel() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    let mut img = image::RgbImage::new(4, 2);
    img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    img.save(&src).unwrap();
    let back = image::open(run(&src, json!({ "rotate": true, "flipH": true })).unwrap())
        .unwrap()
        .to_rgb8();
    assert_eq!(back.get_pixel(3, 0).0, [255, 0, 0]);
    assert_eq!(back.get_pixel(0, 0).0, [0, 0, 0]);
}

#[test]
fn nothing_selected_is_invalid() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(4, 4).save(&src).unwrap();
    let r = run(&src, json!({}));
    assert!(
        matches!(r, Err(ToolError::InvalidOptions(m)) if m == "회전·자르기 중 하나 이상 켜세요.")
    );
}

#[test]
fn jpeg_r90_is_lossless_when_mcu_aligned() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    let original = jpeg(64, 32);
    std::fs::write(&src, &original).unwrap();
    let once = run(&src, json!({ "rotate": true, "angle": "r90" })).unwrap();
    assert_eq!(dims(&once), (32, 64));
    let back = run(&once, json!({ "rotate": true, "angle": "r270" })).unwrap();
    let a = image::load_from_memory(&original).unwrap().to_rgb8();
    assert_eq!(
        image::open(back).unwrap().to_rgb8(),
        a,
        "무손실 왕복이면 픽셀이 같아야 함"
    );
}

#[test]
fn jpeg_unaligned_falls_back() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    std::fs::write(&src, jpeg(65, 33)).unwrap();
    assert_eq!(
        dims(&run(&src, json!({ "rotate": true, "angle": "r90" })).unwrap()),
        (33, 65)
    );
}

#[test]
fn jpeg_with_orientation_falls_back_and_keeps_visual() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    jpeg_with_exif(&src, 40, 16, 6, false);
    assert_eq!(
        dims(&run(&src, json!({ "rotate": true, "angle": "r90" })).unwrap()),
        (40, 16)
    );
}

// ---------- 크롭 (옛 크롭 도구에서 옮김) ----------

fn opts(v: Value) -> EditOptions {
    parse_options(&v).unwrap()
}

#[test]
fn crop_rect_table() {
    let r = |w, h, v| crop_rect(w, h, &opts(v));
    assert_eq!(
        r(
            400,
            300,
            json!({"crop": true, "cropMode": "pixels", "width": 300, "height": 300, "anchor": "center"})
        ),
        (50, 0, 300, 300)
    );
    assert_eq!(
        r(
            400,
            300,
            json!({"crop": true, "cropMode": "pixels", "width": 400, "height": 225, "anchor": "top"})
        ),
        (0, 0, 400, 225)
    );
    assert_eq!(
        r(
            400,
            300,
            json!({"crop": true, "cropMode": "pixels", "width": 500, "height": 100, "anchor": "bottomRight"})
        ),
        (0, 200, 400, 100)
    );
    assert_eq!(
        r(
            300,
            400,
            json!({"crop": true, "cropMode": "pixels", "width": 300, "height": 225, "anchor": "bottom"})
        ),
        (0, 175, 300, 225)
    );
}

#[test]
fn crop_writes_box_size() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(400, 300).save(&src).unwrap();
    assert_eq!(
        dims(
            &run(
                &src,
                json!({"crop": true, "cropMode": "pixels", "width": 400, "height": 225})
            )
            .unwrap()
        ),
        (400, 225)
    );
}

#[test]
fn rotate_then_crop_order() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(40, 20).save(&src).unwrap();
    let out = run(
        &src,
        // 회전 뒤(20×40)에 자르므로 20×30이 나온다 (먼저 자르면 40×20 안이라 20×20)
        json!({"rotate": true, "angle": "r90", "crop": true, "cropMode": "pixels", "width": 20, "height": 30}),
    )
    .unwrap();
    assert_eq!(dims(&out), (20, 30));
}

#[test]
fn schema_links_fields() {
    let s = tool().options_schema();
    assert_eq!(
        s["properties"]["ratio"]["x-enabled-when"],
        json!({ "cropMode": ["manual"] })
    );
    let m = tool().meta();
    assert_eq!(
        (m.id.as_str(), m.name.as_str()),
        ("image.edit", "자르기 · 회전")
    );
}

#[test]
fn schema_has_enum_defaults() {
    let s = tool().options_schema();
    assert_eq!(s["properties"]["angle"]["default"], "none");
    // 묶음 스위치는 꺼짐, 자르기 방식은 직접 지정
    assert_eq!(s["properties"]["rotate"]["default"], false);
    assert_eq!(s["properties"]["crop"]["default"], false);
    assert_eq!(s["properties"]["cropMode"]["default"], "manual");
    assert_eq!(s["properties"]["crop"]["x-group-toggle"], true);
    assert_eq!(s["properties"]["anchor"]["default"], "center");
}

#[test]
fn schema_asks_for_result_preview() {
    assert_eq!(tool().options_schema()["x-preview"], "result");
}

#[test]
fn schema_declares_single_input_and_region() {
    let s = tool().options_schema();
    assert_eq!(s["x-single-input"], true);
    assert_eq!(
        s["x-region"],
        json!({ "x": "cropX", "y": "cropY", "w": "width", "h": "height", "ratio": "ratio", "angle": "angle", "flipH": "flipH", "flipV": "flipV", "when": { "crop": [true], "cropMode": ["manual"] } })
    );
    // 영역 키는 실제 옵션 필드여야 한다
    for k in ["cropX", "cropY", "width", "height", "ratio"] {
        assert!(s["properties"][k].is_object(), "{k}");
    }
}

// ---------- 직접 지정 크롭 ----------

#[test]
fn manual_crop_uses_given_rect() {
    let r = crop_rect(
        400,
        300,
        &opts(
            json!({"crop": true, "cropMode": "manual", "cropX": 10, "cropY": 20, "width": 100, "height": 50}),
        ),
    );
    assert_eq!(r, (10, 20, 100, 50));
}

#[test]
fn manual_crop_clamps_to_image() {
    let r = |v| crop_rect(400, 300, &opts(v));
    // 오른쪽·아래로 넘치면 원본 끝까지
    assert_eq!(
        r(
            json!({"crop": true, "cropMode": "manual", "cropX": 350, "cropY": 250, "width": 100, "height": 100})
        ),
        (350, 250, 50, 50)
    );
    // 시작점이 원본 밖이면 마지막 1px
    assert_eq!(
        r(
            json!({"crop": true, "cropMode": "manual", "cropX": 999, "cropY": 999, "width": 10, "height": 10})
        ),
        (399, 299, 1, 1)
    );
}

#[test]
fn manual_crop_is_after_rotation() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(40, 20).save(&src).unwrap();
    // 90° 회전 뒤 20×40 기준으로 아래쪽 20×10
    let out = run(
        &src,
        json!({"rotate": true, "angle": "r90", "crop": true, "cropMode": "manual", "cropX": 0, "cropY": 30, "width": 20, "height": 10}),
    )
    .unwrap();
    assert_eq!(dims(&out), (20, 10));
}

#[test]
fn lossless_rotate_keeps_icc_when_stripping() {
    use img_parts::{Bytes, DynImage, ImageICC};
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    let mut j = DynImage::from_bytes(Bytes::from(jpeg(64, 32)))
        .unwrap()
        .unwrap();
    j.set_icc_profile(Some(Bytes::from_static(b"fake-icc-profile")));
    let mut bytes = Vec::new();
    j.encoder().write_to(&mut bytes).unwrap();
    std::fs::write(&src, bytes).unwrap();
    let out = run(&src, json!({ "rotate": true, "angle": "r90" })).unwrap();
    assert_eq!(dims(&out), (32, 64));
    assert!(
        waple_tools_image_common::load(&out)
            .unwrap()
            .meta
            .icc
            .is_some()
    );
}

#[test]
fn schema_requires_some_change() {
    let s = tool().options_schema();
    assert_eq!(s["x-require-any"], json!(["rotate", "crop"]));
    assert_eq!(s["x-require-message"], "회전·자르기 중 하나 이상 켜세요.");
}
