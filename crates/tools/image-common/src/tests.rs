use std::path::Path;

use exif::{In, Reader, Tag};
use waple_tools_core::ToolError;

use crate::testkit::{alpha_png, animated_gif, jpeg_with_exif, photo};
use crate::*;

fn exif_fields(raw: &[u8]) -> exif::Exif {
    Reader::new().read_raw(raw.to_vec()).unwrap()
}

#[test]
fn fmt_from_path() {
    assert_eq!(Fmt::from_path(Path::new("a.JPG")), Some(Fmt::Jpeg));
    assert_eq!(Fmt::from_path(Path::new("b.tif")), Some(Fmt::Tiff));
    assert_eq!(Fmt::from_path(Path::new("c.txt")), None);
    assert_eq!(Fmt::Jpeg.ext(), "jpg");
}

#[test]
fn load_applies_orientation() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.jpg");
    jpeg_with_exif(&p, 40, 20, 6, false);
    let l = load(&p).unwrap();
    assert_eq!((l.image.width(), l.image.height()), (20, 40));
    assert_eq!(l.format, Fmt::Jpeg);
}

#[test]
fn load_reads_exif_and_keeps_gps() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.jpg");
    jpeg_with_exif(&p, 8, 8, 1, true);
    let exif = load(&p).unwrap().meta.exif.expect("EXIF 없음");
    assert!(
        exif_fields(&exif)
            .get_field(Tag::GPSLatitudeRef, In::PRIMARY)
            .is_some()
    );
}

#[test]
fn load_rejects_broken_file() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("x.jpg");
    std::fs::write(&p, "not image").unwrap();
    assert!(matches!(load(&p), Err(ToolError::Failed(m)) if m == "이미지를 읽을 수 없습니다."));
}

#[test]
fn encode_roundtrip_each_format() {
    let img = photo(64, 48);
    for (fmt, lossless, want) in [
        (Fmt::Jpeg, false, image::ImageFormat::Jpeg),
        (Fmt::Png, false, image::ImageFormat::Png),
        (Fmt::Webp, false, image::ImageFormat::WebP),
        (Fmt::Webp, true, image::ImageFormat::WebP),
    ] {
        let bytes = encode(
            &img,
            fmt,
            Quality {
                quality: 80,
                lossless,
            },
        )
        .unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), want);
        let back = image::load_from_memory(&bytes).unwrap();
        assert_eq!((back.width(), back.height()), (64, 48));
    }
}

#[test]
fn jpeg_quality_affects_size() {
    let img = photo(256, 256);
    let low = encode(
        &img,
        Fmt::Jpeg,
        Quality {
            quality: 30,
            lossless: false,
        },
    )
    .unwrap();
    let high = encode(
        &img,
        Fmt::Jpeg,
        Quality {
            quality: 90,
            lossless: false,
        },
    )
    .unwrap();
    assert!(low.len() < high.len(), "{} {}", low.len(), high.len());
}

#[test]
fn flatten_uses_background() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.png");
    alpha_png(&p, 4, 4);
    let img = image::open(&p).unwrap();
    let flat = flatten(&img, [255, 0, 0]).to_rgb8();
    assert_eq!(flat.get_pixel(0, 0).0, [255, 0, 0]);
}

#[test]
fn exif_reset_orientation_sets_one() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.jpg");
    jpeg_with_exif(&p, 8, 8, 6, true);
    let exif = load(&p).unwrap().meta.exif.unwrap();
    let fixed = exif_reset_orientation(&exif);
    let f = exif_fields(&fixed);
    let o = f.get_field(Tag::Orientation, In::PRIMARY).unwrap();
    assert_eq!(o.value.get_uint(0), Some(1));
    assert!(
        f.get_field(Tag::GPSLatitudeRef, In::PRIMARY).is_some(),
        "다른 태그는 남아야 함"
    );
}

#[test]
fn exif_without_gps_removes_gps() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.jpg");
    jpeg_with_exif(&p, 8, 8, 3, true);
    let exif = load(&p).unwrap().meta.exif.unwrap();
    let f = exif_fields(&exif_without_gps(&exif).unwrap());
    assert!(f.get_field(Tag::GPSLatitudeRef, In::PRIMARY).is_none());
    assert!(f.get_field(Tag::Orientation, In::PRIMARY).is_some());
}

#[test]
fn with_meta_inserts_exif_into_png_and_webp() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.jpg");
    jpeg_with_exif(&p, 8, 8, 1, true);
    let meta = load(&p).unwrap().meta;
    for fmt in [Fmt::Png, Fmt::Webp, Fmt::Jpeg] {
        let bytes = encode(
            &photo(8, 8),
            fmt,
            Quality {
                quality: 80,
                lossless: false,
            },
        )
        .unwrap();
        let out = with_meta(bytes, fmt, &meta).unwrap();
        image::load_from_memory(&out).unwrap();
        let back = std::fs::write(d.path().join(format!("o.{}", fmt.ext())), &out);
        back.unwrap();
        let reloaded = load(&d.path().join(format!("o.{}", fmt.ext()))).unwrap();
        assert!(reloaded.meta.exif.is_some(), "{fmt:?}에 EXIF 없음");
    }
}

#[test]
fn write_output_names_by_stem() {
    let d = tempfile::tempdir().unwrap();
    let name = write_output(d.path(), Path::new("C:/x/사진 1.png"), Fmt::Webp, b"abc").unwrap();
    assert_eq!(name, Path::new("사진 1.webp"));
    assert_eq!(std::fs::read(d.path().join("사진 1.webp")).unwrap(), b"abc");
}

#[test]
fn display_image_is_downscaled() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("big.png");
    photo(4000, 3000).save(&p).unwrap();
    let shown = display_image(&p, 1600).unwrap();
    assert_eq!(
        (shown.width, shown.height),
        (4000, 3000),
        "원본 크기를 알려 줌"
    );
    let back = image::load_from_memory(&shown.jpeg).unwrap();
    assert_eq!((back.width(), back.height()), (1600, 1200));
    assert!(shown.jpeg.len() < 1_000_000, "{} bytes", shown.jpeg.len());
}

#[test]
fn display_image_flattens_alpha() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.png");
    alpha_png(&p, 8, 8);
    let shown = display_image(&p, 1600).unwrap();
    let px = image::load_from_memory(&shown.jpeg)
        .unwrap()
        .to_rgb8()
        .get_pixel(0, 0)
        .0;
    assert!(px.iter().all(|c| *c > 240), "투명은 흰색: {px:?}");
}

#[test]
fn display_image_shows_first_frame_of_animated_gif() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.gif");
    animated_gif(&p, 20, 10, 3);
    assert!(load(&p).is_err(), "다른 도구용 load는 계속 거부");
    let shown = display_image(&p, 1600).unwrap();
    assert_eq!((shown.width, shown.height), (20, 10));
    let px = image::load_from_memory(&shown.jpeg)
        .unwrap()
        .to_rgb8()
        .get_pixel(10, 5)
        .0;
    assert!(px[2] > 200 && px[0] < 50, "첫 프레임(파랑): {px:?}");
}

#[test]
fn resize_rgba_keeps_edge_color() {
    use image::imageops::FilterType;
    use image::{Rgba, RgbaImage};
    // 투명(검정) 배경 위 흰 띠 3px: 반으로 줄이면 가장자리가 반투명이 된다
    let img = RgbaImage::from_fn(8, 8, |x, _| {
        if (2..5).contains(&x) {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    });
    let half = resize_rgba(&img, 4, 4, FilterType::Lanczos3);
    let edges: Vec<_> = half
        .pixels()
        .filter(|p| (32..255).contains(&p[3]))
        .collect();
    assert!(!edges.is_empty(), "반투명 가장자리가 있어야 함");
    for p in edges {
        assert!(p.0[..3].iter().all(|&c| c > 240), "흰색 유지: {p:?}");
    }
}

#[test]
fn rgb_icc_only_drops_exif_and_non_rgb_profiles() {
    let icc = |space: &[u8; 4]| {
        let mut p = vec![0u8; 128];
        p[16..20].copy_from_slice(space);
        p
    };
    let meta = |p: Vec<u8>| Meta {
        exif: Some(vec![1, 2, 3]),
        icc: Some(p),
    };
    let rgb = rgb_icc_only(&meta(icc(b"RGB ")));
    assert!(rgb.exif.is_none());
    assert_eq!(rgb.icc, Some(icc(b"RGB ")));
    assert!(rgb_icc_only(&meta(icc(b"GRAY"))).icc.is_none());
    assert!(rgb_icc_only(&meta(icc(b"CMYK"))).icc.is_none());
    assert!(
        rgb_icc_only(&meta(vec![0; 10])).icc.is_none(),
        "잘린 프로파일"
    );
}
