use std::cell::Cell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use exif::{In, Reader, Tag};
use localtools_core::{JobContext, ToolError, ToolOutput};
use localtools_image_common::testkit::{alpha_png, jpeg_with_exif, photo};
use localtools_image_common::{Fmt, Quality, encode, load, parse_options};
use serde_json::{Value, json};

use crate::quality::search_quality;
use crate::size::target_size;
use crate::{OptimizeOptions, tool};

fn opts(v: Value) -> OptimizeOptions {
    parse_options(&v).unwrap()
}

/// 엔진처럼 출력 폴더를 따로 두고 실행해 결과 경로를 돌려준다
fn run(input: &Path, v: Value) -> Result<PathBuf, ToolError> {
    let out = input.parent().unwrap().join(format!("out-{}", rand_id()));
    std::fs::create_dir_all(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    match tool().run(&ctx, &[input.to_path_buf()], &out, &v)? {
        ToolOutput::Files(f) => Ok(out.join(&f[0])),
        other => panic!("{other:?}"),
    }
}

fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

fn dims(p: &Path) -> (u32, u32) {
    let i = image::open(p).unwrap();
    (i.width(), i.height())
}

fn jpeg_at(img: &image::DynamicImage, q: u8) -> Result<Vec<u8>, ToolError> {
    encode(
        img,
        Fmt::Jpeg,
        Quality {
            quality: q,
            lossless: false,
        },
    )
}

// ---------- 크기 계산 (옛 리사이즈 도구에서 옮김) ----------

#[test]
fn target_size_table() {
    let t = |w, h, v: Value| {
        let mut v = v;
        v["resize"] = json!(true);
        target_size(w, h, &opts(v))
    };
    assert_eq!(
        t(4000, 3000, json!({"size": "width", "width": 1920})),
        (1920, 1440)
    );
    assert_eq!(
        t(4000, 3000, json!({"size": "height", "height": 1080})),
        (1440, 1080)
    );
    assert_eq!(
        t(
            4000,
            3000,
            json!({"size": "exact", "width": 1000, "height": 1000, "fit": "contain"})
        ),
        (1000, 750)
    );
    assert_eq!(
        t(
            4000,
            3000,
            json!({"size": "exact", "width": 1000, "height": 1000, "fit": "stretch"})
        ),
        (1000, 1000)
    );
    assert_eq!(
        t(
            4000,
            3000,
            json!({"size": "exact", "width": 1000, "height": 1000, "fit": "cover"})
        ),
        (1000, 1000)
    );
    assert_eq!(
        t(
            4000,
            3000,
            json!({"size": "exact", "width": 1000, "height": 1000, "fit": "pad"})
        ),
        (1000, 1000)
    );
    assert_eq!(
        t(4000, 3000, json!({"size": "preset", "preset": "og"})),
        (1200, 630)
    );
}

#[test]
fn no_upscale_keeps_size() {
    let t = |w, h, v: Value| target_size(w, h, &opts(v));
    assert_eq!(
        t(
            800,
            600,
            json!({"resize": true, "size": "width", "width": 1920})
        ),
        (800, 600)
    );
    assert_eq!(
        t(
            800,
            600,
            json!({"resize": true, "size": "width", "width": 1920, "noUpscale": false})
        ),
        (1920, 1440)
    );
}

#[test]
fn huge_size_is_capped() {
    let o = opts(json!({"resize": true, "size": "width", "width": 999999, "noUpscale": false}));
    assert_eq!(target_size(100, 100, &o), (20000, 20000));
}

#[test]
fn resize_off_ignores_width() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(40, 30).save(&src).unwrap();
    let out = run(
        &src,
        json!({"resize": false, "width": 10, "compress": true}),
    )
    .unwrap();
    assert_eq!(dims(&out), (40, 30));
}

#[test]
fn preset_og_is_1200x630() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    std::fs::write(&src, jpeg_at(&photo(2400, 1600), 90).unwrap()).unwrap();
    let out = run(
        &src,
        json!({"resize": true, "size": "preset", "preset": "og", "compress": false}),
    )
    .unwrap();
    assert_eq!(dims(&out), (1200, 630));
}

#[test]
fn cover_crops_to_exact_box() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(400, 300).save(&src).unwrap();
    let out = run(
        &src,
        json!({"resize": true, "size": "exact", "width": 200, "height": 200, "fit": "cover"}),
    )
    .unwrap();
    assert_eq!(dims(&out), (200, 200));
}

#[test]
fn pad_fills_box_with_background() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(400, 300).save(&src).unwrap();
    let v = json!({"resize": true, "size": "exact", "width": 300, "height": 300, "fit": "pad",
        "convert": true, "format": "jpg", "background": "#ff0000"});
    let out = run(&src, v).unwrap();
    assert_eq!(dims(&out), (300, 300));
    let px = image::open(&out).unwrap().to_rgb8().get_pixel(150, 5).0;
    assert!(
        px[0] > 230 && px[1] < 30 && px[2] < 30,
        "위 띠가 배경색이어야 함: {px:?}"
    );
}

// ---------- 단계 스위치 ----------

#[test]
fn default_has_no_step_on() {
    // 처음에는 아무 단계도 켜져 있지 않다: 그대로 실행하면 거부
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    std::fs::write(&src, jpeg_at(&photo(64, 48), 98).unwrap()).unwrap();
    let r = run(&src, json!({}));
    assert!(
        matches!(&r, Err(ToolError::InvalidOptions(m)) if m == "적용할 단계를 하나 이상 켜세요."),
        "{r:?}"
    );
}

#[test]
fn compress_only_keeps_size_and_shrinks() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    std::fs::write(&src, jpeg_at(&photo(512, 384), 98).unwrap()).unwrap();
    let out = run(&src, json!({"compress": true, "quality": 80})).unwrap();
    assert_eq!(out.file_name().unwrap(), "a.jpg");
    assert_eq!(dims(&out), (512, 384));
    assert!(std::fs::metadata(&out).unwrap().len() < std::fs::metadata(&src).unwrap().len());
}

#[test]
fn resize_only_keeps_quality_high() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(400, 300).save(&src).unwrap();
    let out = run(
        &src,
        json!({"resize": true, "size": "width", "width": 200, "compress": false,
        "convert": true, "format": "jpg"}),
    )
    .unwrap();
    let resized =
        image::open(&src)
            .unwrap()
            .resize_exact(200, 150, image::imageops::FilterType::Lanczos3);
    let at95 = jpeg_at(&resized, 95).unwrap().len() as f64;
    let got = std::fs::metadata(&out).unwrap().len() as f64;
    assert!(
        (got - at95).abs() / at95 < 0.01,
        "품질 95로 저장돼야 함: {got} vs {at95}"
    );
}

#[test]
fn convert_only_png_to_webp() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(32, 24).save(&src).unwrap();
    let out = run(
        &src,
        json!({"convert": true, "format": "webp", "compress": false}),
    )
    .unwrap();
    assert_eq!(out.file_name().unwrap(), "a.webp");
    assert_eq!(dims(&out), (32, 24));
}

#[test]
fn nothing_on_is_invalid() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(4, 4).save(&src).unwrap();
    let r = run(
        &src,
        json!({"resize": false, "convert": false, "compress": false}),
    );
    assert!(
        matches!(r, Err(ToolError::InvalidOptions(m)) if m == "적용할 단계를 하나 이상 켜세요.")
    );
}

// ---------- 포맷 (옛 포맷 변환·WebP 도구에서 옮김) ----------

#[test]
fn transparent_png_to_jpg_uses_background() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    alpha_png(&src, 32, 32);
    let out = run(
        &src,
        json!({"convert": true, "format": "jpg", "background": "#ff0000"}),
    )
    .unwrap();
    let px = image::open(out).unwrap().to_rgb8().get_pixel(2, 2).0;
    assert!(px[0] > 245 && px[1] < 10 && px[2] < 10, "{px:?}");
}

#[test]
fn bad_background_is_invalid_options() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(4, 4).save(&src).unwrap();
    let r = run(
        &src,
        json!({"convert": true, "format": "jpg", "background": "red"}),
    );
    assert!(
        matches!(r, Err(ToolError::InvalidOptions(m)) if m == "배경색은 #rrggbb 형식이어야 합니다.")
    );
}

#[test]
fn webp_lossless_is_pixel_exact() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    let original = photo(16, 16);
    original.save(&src).unwrap();
    let out = run(
        &src,
        json!({"convert": true, "format": "webp", "lossless": true}),
    )
    .unwrap();
    assert_eq!(image::open(out).unwrap().to_rgb8(), original.to_rgb8());
}

#[test]
fn lossless_ignores_quality() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    let original = photo(16, 16);
    original.save(&src).unwrap();
    let out = run(
        &src,
        json!({"convert": true, "format": "webp", "lossless": true, "quality": 1}),
    )
    .unwrap();
    assert_eq!(image::open(out).unwrap().to_rgb8(), original.to_rgb8());
}

#[test]
fn keep_metadata_resets_orientation() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    jpeg_with_exif(&src, 16, 8, 6, false);
    let out = run(
        &src,
        json!({"convert": true, "format": "webp", "metadata": "keep"}),
    )
    .unwrap();
    let loaded = load(&out).unwrap();
    assert_eq!((loaded.image.width(), loaded.image.height()), (8, 16));
    let exif = Reader::new().read_raw(loaded.meta.exif.unwrap()).unwrap();
    assert_eq!(
        exif.get_field(Tag::Orientation, In::PRIMARY)
            .unwrap()
            .value
            .get_uint(0),
        Some(1)
    );
}

#[test]
fn strip_by_default() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    jpeg_with_exif(&src, 16, 8, 1, true);
    let out = run(&src, json!({"compress": true})).unwrap();
    assert!(load(&out).unwrap().meta.exif.is_none());
}

#[test]
fn gif_input_saves_png() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.gif");
    photo(8, 8).to_rgba8().save(&src).unwrap();
    let out = run(&src, json!({"compress": true})).unwrap();
    assert_eq!(out.file_name().unwrap(), "a.png");
}

// ---------- 용량 (옛 압축 도구에서 옮김) ----------

#[test]
fn search_quality_hits_target_within_5_percent() {
    let img = photo(1024, 768);
    let lo = jpeg_at(&img, 40).unwrap().len();
    let hi = jpeg_at(&img, 80).unwrap().len();
    let target = (lo + hi) / 2;
    let out = search_quality(target, |q| jpeg_at(&img, q)).unwrap();
    assert!(out.len() <= target);
    assert!((target - out.len()) as f64 / target as f64 <= 0.05);
}

#[test]
fn search_quality_encodes_at_most_8_times() {
    let img = photo(256, 256);
    let calls = Cell::new(0);
    let target = jpeg_at(&img, 55).unwrap().len();
    search_quality(target, |q| {
        calls.set(calls.get() + 1);
        jpeg_at(&img, q)
    })
    .unwrap();
    assert!(calls.get() <= 8);
}

#[test]
fn target_kb_unreachable_returns_smallest() {
    let img = photo(128, 128);
    let out = search_quality(1, |q| jpeg_at(&img, q)).unwrap();
    assert_eq!(out.len(), jpeg_at(&img, 1).unwrap().len());
}

#[test]
fn png_is_lossless() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    let img = photo(64, 64);
    img.save(&src).unwrap();
    let out = run(&src, json!({"compress": true})).unwrap();
    assert_eq!(image::open(out).unwrap().to_rgb8(), img.to_rgb8());
}

fn distinct_colors(p: &Path) -> usize {
    let img = image::open(p).unwrap().to_rgba8();
    img.pixels().map(|p| p.0).collect::<HashSet<_>>().len()
}

#[test]
fn png_colors_reduces_palette() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    photo(128, 128).save(&src).unwrap();
    let reduced = run(&src, json!({"compress": true, "pngColors": 16})).unwrap();
    let full = run(&src, json!({"compress": true})).unwrap();
    assert!(distinct_colors(&reduced) <= 16);
    assert!(std::fs::metadata(&reduced).unwrap().len() < std::fs::metadata(&full).unwrap().len());
}

#[test]
fn png_colors_keeps_binary_transparency() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    alpha_png(&src, 32, 32);
    let out = run(&src, json!({"compress": true, "pngColors": 8})).unwrap();
    let img = image::open(out).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(0, 0).0[3], 0);
    assert_eq!(img.get_pixel(31, 0).0[3], 255);
}

// ---------- 스키마 ----------

#[test]
fn schema_has_group_toggles_and_require_any() {
    let s = tool().options_schema();
    for k in ["resize", "convert", "compress"] {
        assert_eq!(s["properties"][k]["x-group-toggle"], true, "{k}");
    }
    assert_eq!(s["x-require-any"], json!(["resize", "convert", "compress"]));
    assert_eq!(s["properties"]["quality"]["minimum"], 1);
    assert_eq!(s["properties"]["quality"]["maximum"], 100);
    assert_eq!(s["properties"]["background"]["format"], "color");
}

#[test]
fn meta_matches_catalog() {
    let m = tool().meta();
    assert_eq!(
        (m.id.as_str(), m.name.as_str()),
        ("image.optimize", "이미지 최적화")
    );
}

#[test]
fn schema_links_fields() {
    let s = tool().options_schema();
    assert_eq!(
        s["properties"]["quality"]["x-enabled-when"],
        // PNG·무손실 WebP로 바꾸면 숨김
        json!([
            { "compress": [true], "convert": [false], "mode": ["quality"] },
            { "compress": [true], "convert": [true], "format": ["jpg"], "mode": ["quality"] },
            { "compress": [true], "convert": [true], "format": ["webp"], "lossless": [false], "mode": ["quality"] }
        ])
    );
    assert_eq!(
        s["properties"]["background"]["x-enabled-when"],
        json!([{ "convert": [true], "format": ["jpg"] }, { "resize": [true], "fit": ["pad"] }])
    );
    // PNG 색상 수는 PNG로 저장될 때만 (WebP·JPG로 바꾸면 꺼짐)
    assert_eq!(
        s["properties"]["pngColors"]["x-enabled-when"],
        json!([
            { "compress": [true], "convert": [false], "$inputExt": ["png"] },
            { "compress": [true], "convert": [true], "format": ["png"] }
        ])
    );
    assert_eq!(s["properties"]["width"]["x-group"], "크기");
}

#[test]
fn schema_has_enum_defaults() {
    let s = tool().options_schema();
    assert_eq!(s["properties"]["size"]["default"], "width");
    assert_eq!(s["properties"]["fit"]["default"], "cover");
    assert_eq!(s["properties"]["format"]["default"], "webp");
    assert_eq!(s["properties"]["mode"]["default"], "quality");
    assert_eq!(s["properties"]["metadata"]["default"], "strip");
}

// ---------- 리뷰 수정 ----------

#[test]
fn resize_off_ignores_pad() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    // 16비트 RGB: 크기 단계를 거치지 않으면 비트 심도·채널이 그대로여야 한다
    image::DynamicImage::ImageRgb16(image::ImageBuffer::from_pixel(
        20,
        10,
        image::Rgb([1000u16, 2000, 3000]),
    ))
    .save(&src)
    .unwrap();
    let out = run(
        &src,
        json!({"resize": false, "size": "exact", "fit": "pad", "width": 50, "height": 50,
        "convert": true, "format": "png", "compress": false}),
    )
    .unwrap();
    let img = image::open(&out).unwrap();
    assert_eq!((img.width(), img.height()), (20, 10));
    assert!(
        !img.color().has_alpha(),
        "꺼진 크기 단계의 여백 추가가 적용되면 안 됨"
    );
}

#[test]
fn strip_keeps_icc_profile() {
    use img_parts::{Bytes, DynImage, ImageICC};
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    let mut j = DynImage::from_bytes(Bytes::from(jpeg_at(&photo(32, 32), 90).unwrap()))
        .unwrap()
        .unwrap();
    j.set_icc_profile(Some(Bytes::from_static(b"fake-icc-profile")));
    let mut bytes = Vec::new();
    j.encoder().write_to(&mut bytes).unwrap();
    std::fs::write(&src, bytes).unwrap();
    let out = run(&src, json!({"compress": true})).unwrap();
    let meta = load(&out).unwrap().meta;
    assert!(
        meta.icc.is_some(),
        "메타데이터를 지워도 색상 프로파일은 남아야 색이 안 바뀜"
    );
    assert!(meta.exif.is_none());
}

#[test]
fn animated_gif_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("anim.gif");
    localtools_image_common::testkit::animated_gif(&src, 8, 8, 3);
    let r = run(&src, json!({"compress": true}));
    assert!(
        matches!(r, Err(ToolError::Failed(m)) if m == "움직이는 GIF는 아직 지원하지 않습니다.")
    );
}

#[test]
fn schema_splits_label_and_help() {
    // 주석 전체가 description 하나로 온다: 첫 문단은 라벨, 빈 줄 뒤는 물음표 풍선 설명 (화면이 나눈다)
    let q = &tool().options_schema()["properties"]["quality"];
    let d = q["description"].as_str().unwrap();
    let (label, help) = d
        .split_once(
            "

",
        )
        .unwrap();
    assert_eq!(label, "품질");
    assert!(help.contains("oxipng"));
}

#[test]
fn schema_shows_result_only_when_resizing() {
    assert_eq!(
        tool().options_schema()["x-preview-result-when"],
        json!({ "resize": [true] })
    );
}
