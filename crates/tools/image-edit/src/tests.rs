use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use localtools_core::{JobContext, ToolError, ToolOutput};
use localtools_image_common::testkit::{jpeg_with_exif, photo};
use localtools_image_common::{Fmt, Quality, encode, parse_options};
use serde_json::{Value, json};

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
        dims(&run(&src, json!({ "angle": "r90" })).unwrap()),
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
    let back = image::open(run(&src, json!({ "flipH": true })).unwrap())
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
        matches!(r, Err(ToolError::InvalidOptions(m)) if m == "회전·크롭·여백 중 하나 이상 고르세요.")
    );
}

#[test]
fn jpeg_r90_is_lossless_when_mcu_aligned() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    let original = jpeg(64, 32);
    std::fs::write(&src, &original).unwrap();
    let once = run(&src, json!({ "angle": "r90" })).unwrap();
    assert_eq!(dims(&once), (32, 64));
    let back = run(&once, json!({ "angle": "r270" })).unwrap();
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
        dims(&run(&src, json!({ "angle": "r90" })).unwrap()),
        (33, 65)
    );
}

#[test]
fn jpeg_with_orientation_falls_back_and_keeps_visual() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    jpeg_with_exif(&src, 40, 16, 6, false);
    assert_eq!(
        dims(&run(&src, json!({ "angle": "r90" })).unwrap()),
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
            json!({"crop": "ratio", "ratio": "r1x1", "anchor": "center"})
        ),
        (50, 0, 300, 300)
    );
    assert_eq!(
        r(
            400,
            300,
            json!({"crop": "ratio", "ratio": "r16x9", "anchor": "top"})
        ),
        (0, 0, 400, 225)
    );
    assert_eq!(
        r(
            400,
            300,
            json!({"crop": "pixels", "width": 500, "height": 100, "anchor": "bottomRight"})
        ),
        (0, 200, 400, 100)
    );
    assert_eq!(
        r(
            300,
            400,
            json!({"crop": "ratio", "ratio": "r4x3", "anchor": "bottom"})
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
        dims(&run(&src, json!({"crop": "ratio", "ratio": "r16x9"})).unwrap()),
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
        json!({"angle": "r90", "crop": "ratio", "ratio": "r1x1"}),
    )
    .unwrap();
    assert_eq!(dims(&out), (20, 20));
}

// ---------- 여백 ----------

/// 가운데 w×h는 불투명 파랑, 바깥 `border`px는 투명인 PNG
fn bordered_png(path: &Path, w: u32, h: u32, border: u32) {
    let img = RgbaImage::from_fn(w + border * 2, h + border * 2, |x, y| {
        let inside = x >= border && x < border + w && y >= border && y < border + h;
        if inside {
            Rgba([0, 0, 255, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    });
    img.save(path).unwrap();
}

#[test]
fn trim_removes_transparent_border() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    bordered_png(&src, 30, 30, 10);
    assert_eq!(
        dims(&run(&src, json!({"margin": "trim"})).unwrap()),
        (30, 30)
    );
}

#[test]
fn trim_removes_solid_border() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    let img = image::RgbImage::from_fn(60, 40, |x, y| {
        if (10..50).contains(&x) && (10..30).contains(&y) {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    });
    img.save(&src).unwrap();
    assert_eq!(
        dims(&run(&src, json!({"margin": "trim"})).unwrap()),
        (40, 20)
    );
}

#[test]
fn trim_of_blank_image_keeps_original() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    image::RgbImage::from_pixel(20, 10, image::Rgb([255, 255, 255]))
        .save(&src)
        .unwrap();
    assert_eq!(
        dims(&run(&src, json!({"margin": "trim"})).unwrap()),
        (20, 10)
    );
}

#[test]
fn pad_adds_pixels() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(40, 30).save(&src).unwrap();
    let out = run(&src, json!({"margin": "pad", "padding": 20})).unwrap();
    assert_eq!(dims(&out), (80, 70));
    assert_eq!(
        image::open(out).unwrap().to_rgba8().get_pixel(0, 0).0[3],
        0,
        "PNG는 투명 여백"
    );
}

#[test]
fn pad_jpg_uses_background() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    std::fs::write(&src, jpeg(40, 32)).unwrap();
    let out = run(
        &src,
        json!({"margin": "pad", "padding": 16, "background": "#ff0000"}),
    )
    .unwrap();
    assert_eq!(dims(&out), (72, 64));
    let px = image::open(out).unwrap().to_rgb8().get_pixel(2, 2).0;
    assert!(px[0] > 230 && px[1] < 30 && px[2] < 30, "{px:?}");
}

#[test]
fn square_canvas_centers() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(400, 300).save(&src).unwrap();
    let out = run(&src, json!({"margin": "square"})).unwrap();
    assert_eq!(dims(&out), (400, 400));
    let img = image::open(out).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(200, 10).0[3], 0, "위 50px는 투명");
    assert_eq!(img.get_pixel(200, 200).0[3], 255);
}

#[test]
fn schema_links_fields() {
    let s = tool().options_schema();
    assert_eq!(
        s["properties"]["tolerance"]["x-enabled-when"],
        json!({ "margin": ["trim"] })
    );
    assert_eq!(
        s["properties"]["ratio"]["x-enabled-when"],
        json!({ "crop": ["ratio"] })
    );
    let m = tool().meta();
    assert_eq!(
        (m.id.as_str(), m.name.as_str()),
        ("image.edit", "자르기 · 회전 · 여백")
    );
}
