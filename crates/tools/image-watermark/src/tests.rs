use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage, Rgba, RgbaImage};
use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::INPUT_EXTS;
use waple_tools_image_common::testkit::{animated_gif, jpeg_with_exif, photo};

use crate::{mark, tool};

const GRAY: [u8; 3] = [128, 128, 128];

fn solid(path: &Path, w: u32, h: u32) {
    RgbImage::from_pixel(w, h, Rgb(GRAY)).save(path).unwrap();
}

/// 빨간 불투명 정사각형 로고 PNG
fn red_logo(dir: &Path, size: u32) -> PathBuf {
    let p = dir.join("logo.png");
    RgbaImage::from_pixel(size, size, Rgba([255, 0, 0, 255]))
        .save(&p)
        .unwrap();
    p
}

fn run(input: &Path, out: &Path, opts: Value) -> Result<PathBuf, ToolError> {
    let ctx = JobContext::new(|_| {}, Default::default());
    std::fs::create_dir_all(out).unwrap();
    let ToolOutput::Files(names) = tool().run(&ctx, &[input.to_path_buf()], out, &opts)? else {
        panic!("파일이어야 함");
    };
    Ok(out.join(&names[0]))
}

/// 원본(회색)과 달라진 픽셀 좌표
fn changed(path: &Path) -> Vec<(u32, u32)> {
    let img = image::open(path).unwrap().to_rgb8();
    img.enumerate_pixels()
        .filter(|(_, _, p)| p.0 != GRAY)
        .map(|(x, y, _)| (x, y))
        .collect()
}

fn logo_opts(logo: &Path, extra: Value) -> Value {
    let mut o =
        json!({ "kind": "logo", "logo": logo, "logoSize": 10, "margin": 0, "opacity": 100 });
    o.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    o
}

/// 이 앱은 Windows 전용: 맑은 고딕이 없는 환경이면 텍스트 테스트를 건너뛴다
fn has_malgun() -> bool {
    let ok = mark::system_font("malgun.ttf").is_ok();
    if !ok {
        eprintln!("맑은 고딕이 없어 건너뜀");
    }
    ok
}

#[test]
fn meta_matches_catalog() {
    let m = tool().meta();
    assert_eq!(m.id, "image.watermark");
    assert_eq!(m.name, "워터마크");
    assert_eq!(m.category, "image");
    assert_eq!(m.description, "텍스트나 로고를 이미지에 넣습니다.");
}

#[test]
fn schema_has_file_field_and_allow_larger() {
    let s = tool().options_schema();
    assert_eq!(s["x-allow-larger"], true);
    let logo = &s["properties"]["logo"];
    assert_eq!(logo["format"], "file");
    assert_eq!(logo["x-file-extensions"], json!(INPUT_EXTS));
    assert_eq!(logo["x-enabled-when"], json!({ "kind": ["logo"] }));
}

#[test]
fn text_draws_hangul() {
    if !has_malgun() {
        return;
    }
    let font = mark::system_font("malgun.ttf").unwrap();
    use ab_glyph::Font as _;
    assert_ne!(font.glyph_id('가').0, 0, "한글 글리프가 있어야 함");

    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 400, 300);
    let out = run(
        &src,
        &d.path().join("out"),
        json!({ "text": "가나다", "color": "#ff0000", "opacity": 100, "position": "center", "textSize": 20 }),
    )
    .unwrap();
    let img = image::open(&out).unwrap().to_rgb8();
    assert!(
        img.pixels().any(|p| p.0 == [255, 0, 0]),
        "글자 속은 정한 색 그대로"
    );
    let px = changed(&out);
    assert!(px.len() > 500, "{}", px.len());
    // 가운데 정렬: 바뀐 픽셀의 중심이 이미지 가운데 근처
    let cx = px.iter().map(|p| p.0 as f32).sum::<f32>() / px.len() as f32;
    assert!((cx - 200.0).abs() < 40.0, "{cx}");
}

#[test]
fn long_text_shrinks_to_fit() {
    if !has_malgun() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 200, 100);
    let long = "워터마크 ".repeat(40);
    let out = run(
        &src,
        &d.path().join("out"),
        json!({ "text": long, "opacity": 100, "textSize": 30, "margin": 10 }),
    )
    .unwrap();
    // 여백 10px 안쪽에만 그린다
    let px = changed(&out);
    assert!(!px.is_empty());
    assert!(
        px.iter()
            .all(|&(x, y)| (10..190).contains(&x) && (10..90).contains(&y))
    );
}

#[test]
fn text_errors() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 10, 10);
    let out = d.path().join("out");
    assert!(matches!(
        run(&src, &out, json!({ "text": "  " })),
        Err(ToolError::InvalidOptions(m)) if m == "넣을 텍스트를 입력해 주세요."
    ));
    assert!(matches!(
        run(&src, &out, json!({ "color": "red" })),
        Err(ToolError::InvalidOptions(_))
    ));
    assert!(matches!(
        mark::system_font("없는글꼴.ttf"),
        Err(ToolError::Failed(m)) if m.contains("글꼴 파일(없는글꼴.ttf)")
    ));
}

#[test]
fn nine_positions_hit_the_right_third() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 300, 300);
    let logo = red_logo(d.path(), 8);
    let cases = [
        ("topLeft", 0, 0),
        ("top", 1, 0),
        ("topRight", 2, 0),
        ("left", 0, 1),
        ("center", 1, 1),
        ("right", 2, 1),
        ("bottomLeft", 0, 2),
        ("bottom", 1, 2),
        ("bottomRight", 2, 2),
    ];
    for (pos, col, row) in cases {
        let out = run(
            &src,
            &d.path().join(pos),
            logo_opts(&logo, json!({ "position": pos })),
        )
        .unwrap();
        let px = changed(&out);
        // 로고 30px (짧은 변 300의 10%), 여백 0
        assert_eq!(px.len(), 30 * 30, "{pos}");
        assert!(
            px.iter().all(|&(x, y)| x / 100 == col && y / 100 == row),
            "{pos}"
        );
    }
}

#[test]
fn margin_moves_away_from_edge() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 200, 100);
    let logo = red_logo(d.path(), 8);
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(&logo, json!({ "position": "topLeft", "margin": 10 })),
    )
    .unwrap();
    // 짧은 변 100의 10% = 10px 띄우고 10px 로고
    let px = changed(&out);
    assert_eq!(px.iter().map(|p| p.0).min(), Some(10));
    assert_eq!(px.iter().map(|p| p.1).min(), Some(10));
}

#[test]
fn tile_spreads_over_image() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 400, 300);
    let logo = red_logo(d.path(), 8);
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(&logo, json!({ "position": "tile" })),
    )
    .unwrap();
    let px = changed(&out);
    // 한 번 넣은 것(30×30)보다 훨씬 많고, 네 귀퉁이 사분면에 모두 있다
    assert!(px.len() > 30 * 30 * 6, "{}", px.len());
    for (qx, qy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert!(
            px.iter().any(|&(x, y)| x / 200 == qx && y / 150 == qy),
            "{qx},{qy}"
        );
    }
}

#[test]
fn opacity_zero_and_full() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 100, 100);
    let logo = red_logo(d.path(), 8);
    let pixel = |opacity: u8| {
        let out = run(
            &src,
            &d.path().join(format!("o{opacity}")),
            logo_opts(&logo, json!({ "position": "center", "opacity": opacity })),
        )
        .unwrap();
        image::open(&out).unwrap().to_rgb8().get_pixel(50, 50).0
    };
    assert_eq!(pixel(0), GRAY, "0이면 원본 그대로");
    assert_eq!(pixel(100), [255, 0, 0], "100이면 로고 색 그대로");
    let half = pixel(50);
    assert!(half[0] > 150 && half[0] < 230 && half[1] < 100, "{half:?}");
}

#[test]
fn logo_with_transparency_keeps_background() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 100, 100);
    // 왼쪽 절반만 불투명 파랑인 로고
    let logo = d.path().join("half.png");
    RgbaImage::from_fn(20, 20, |x, _| {
        if x < 10 {
            Rgba([0, 0, 255, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
    .save(&logo)
    .unwrap();
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(
            &logo,
            json!({ "position": "topLeft", "logoSize": 20, "opacity": 100 }),
        ),
    )
    .unwrap();
    let img = image::open(&out).unwrap().to_rgb8();
    assert_eq!(img.get_pixel(5, 5).0, [0, 0, 255]);
    assert_eq!(img.get_pixel(15, 5).0, GRAY, "투명 부분은 원본");
}

#[test]
fn big_logo_shrinks_to_image() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 120, 60);
    let logo = red_logo(d.path(), 500);
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(&logo, json!({ "logoSize": 100, "position": "center" })),
    )
    .unwrap();
    let img = image::open(&out).unwrap();
    assert_eq!((img.width(), img.height()), (120, 60));
    // 60×60으로 줄어 가운데에
    assert_eq!(changed(&out).len(), 60 * 60);
}

#[test]
fn logo_path_errors() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    solid(&src, 10, 10);
    let out = d.path().join("out");
    let err = |logo: &str| run(&src, &out, json!({ "kind": "logo", "logo": logo })).unwrap_err();
    assert!(matches!(err(""), ToolError::InvalidOptions(m) if m == "로고 파일을 골라 주세요."));
    let missing = d.path().join("없음.png");
    assert!(
        matches!(err(missing.to_str().unwrap()), ToolError::Failed(m) if m == "로고 파일을 찾을 수 없습니다.")
    );
    let txt = d.path().join("logo.png");
    std::fs::write(&txt, "not image").unwrap();
    assert!(
        matches!(err(txt.to_str().unwrap()), ToolError::Failed(m) if m == "로고 파일: 이미지를 읽을 수 없습니다.")
    );
    // 원래 이유를 살린다
    let gif = d.path().join("moving.gif");
    animated_gif(&gif, 4, 4, 3);
    assert!(
        matches!(err(gif.to_str().unwrap()), ToolError::Failed(m) if m.contains("움직이는 GIF"))
    );
}

#[test]
fn huge_logo_is_capped() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("wide.png");
    RgbaImage::from_pixel(5000, 10, Rgba([255, 0, 0, 255]))
        .save(&p)
        .unwrap();
    let logo = mark::load_logo(p.to_str().unwrap()).unwrap();
    assert_eq!(logo.width(), 4096);
}

#[test]
fn sixteen_bit_png_stays_sixteen_bit() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("deep.png");
    image::ImageBuffer::<Rgb<u16>, _>::from_fn(100, 100, |x, _| Rgb([x as u16 * 600, 1000, 30000]))
        .save(&src)
        .unwrap();
    let logo = red_logo(d.path(), 8);
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(&logo, json!({ "position": "center" })),
    )
    .unwrap();
    let img = image::open(&out).unwrap();
    assert_eq!(img.color(), image::ColorType::Rgb16);
    let img = img.to_rgb16();
    assert_eq!(img.get_pixel(50, 50).0, [65535, 0, 0], "워터마크");
    assert_eq!(
        img.get_pixel(1, 1).0,
        [600, 1000, 30000],
        "원본 16비트 값 그대로"
    );
}

#[test]
fn keeps_format_and_source() {
    let d = tempfile::tempdir().unwrap();
    let logo = red_logo(d.path(), 8);
    for (name, fmt) in [
        ("a.jpg", image::ImageFormat::Jpeg),
        ("b.png", image::ImageFormat::Png),
        ("c.webp", image::ImageFormat::WebP),
        ("d.bmp", image::ImageFormat::Bmp),
    ] {
        let src = d.path().join(name);
        photo(64, 48).save_with_format(&src, fmt).unwrap();
        let before = std::fs::read(&src).unwrap();
        let out = run(&src, &d.path().join("out"), logo_opts(&logo, json!({}))).unwrap();
        assert_eq!(
            out.file_name().unwrap(),
            Path::new(name),
            "같은 이름·확장자"
        );
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), fmt, "{name}");
        assert_eq!(std::fs::read(&src).unwrap(), before, "원본 그대로: {name}");
    }
}

#[test]
fn transparent_png_stays_transparent() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.png");
    waple_tools_image_common::testkit::alpha_png(&src, 100, 100);
    let logo = red_logo(d.path(), 8);
    let out = run(
        &src,
        &d.path().join("out"),
        logo_opts(&logo, json!({ "position": "bottomRight" })),
    )
    .unwrap();
    let img = image::open(&out).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(0, 0)[3], 0, "투명은 투명 그대로");
    assert_eq!(img.get_pixel(95, 95).0, [255, 0, 0, 255]);
}

#[test]
fn metadata_keep_or_strip() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("a.jpg");
    jpeg_with_exif(&src, 40, 20, 6, false);
    let logo = red_logo(d.path(), 8);
    let exif_of = |mode: &str| {
        let out = run(
            &src,
            &d.path().join(mode),
            logo_opts(&logo, json!({ "metadata": mode })),
        )
        .unwrap();
        let l = waple_tools_image_common::load(&out).unwrap();
        // EXIF 방향(6)을 픽셀에 반영해 세로로 저장
        assert_eq!((l.image.width(), l.image.height()), (20, 40));
        l.meta.exif.is_some()
    };
    assert!(exif_of("keep"));
    assert!(!exif_of("strip"));
}

#[test]
fn fitted_text_fits_box() {
    if !has_malgun() {
        return;
    }
    let font = mark::system_font("malgun.ttf").unwrap();
    for max_w in (20..400).step_by(7) {
        let px = mark::fit_px(&font, "© 워터마크 Wj", 200.0, max_w, 1000).unwrap();
        let (_, b) = mark::layout(&font, "© 워터마크 Wj", px);
        assert!(
            (b[2] - b[0]).ceil() as u32 <= max_w,
            "{max_w}: {}",
            b[2] - b[0]
        );
    }
}
