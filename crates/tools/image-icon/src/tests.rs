use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::testkit::photo;

use super::*;

/// 입력 → 결과 세트 (상대 경로 → 내용). 모든 경로는 `logo-icons/` 아래
fn run_on(
    img: image::DynamicImage,
    opts: serde_json::Value,
) -> Result<BTreeMap<String, Vec<u8>>, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("logo.png");
    img.save(&src).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, std::slice::from_ref(&src), &out, &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert!(src.is_file(), "원본은 그대로");
    Ok(names
        .iter()
        .map(|n| {
            let rel = n.strip_prefix("logo-icons").expect("logo-icons 아래");
            (
                rel.to_string_lossy().replace('\\', "/"),
                std::fs::read(out.join(n)).unwrap(),
            )
        })
        .collect())
}

fn run(opts: serde_json::Value) -> BTreeMap<String, Vec<u8>> {
    run_on(photo(600, 600), opts).unwrap()
}

fn png(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes).unwrap().to_rgba8()
}

fn side(bytes: &[u8]) -> u32 {
    let img = image::load_from_memory(bytes).unwrap();
    assert_eq!(img.width(), img.height());
    img.width()
}

fn ico_sizes(bytes: &[u8]) -> Vec<u32> {
    assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..n)
        .map(|i| {
            let e = &bytes[6 + 16 * i..];
            let len = u32::from_le_bytes(e[8..12].try_into().unwrap()) as usize;
            let off = u32::from_le_bytes(e[12..16].try_into().unwrap()) as usize;
            let w = if e[0] == 0 { 256 } else { u32::from(e[0]) };
            assert_eq!(side(&bytes[off..off + len]), w, "항목 크기 = 안의 PNG 크기");
            w
        })
        .collect()
}

#[test]
fn favicon_set_matches_spec() {
    let f = run(json!({}));
    let names: Vec<_> = f.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "favicon/android-chrome-192x192.png",
            "favicon/android-chrome-512x512.png",
            "favicon/apple-touch-icon.png",
            "favicon/favicon-16x16.png",
            "favicon/favicon-32x32.png",
            "favicon/favicon.ico",
            "favicon/head.html",
            "favicon/site.webmanifest",
        ]
    );
    assert_eq!(ico_sizes(&f["favicon/favicon.ico"]), [16, 32, 48]);
    assert_eq!(side(&f["favicon/favicon-16x16.png"]), 16);
    assert_eq!(side(&f["favicon/apple-touch-icon.png"]), 180);
    assert_eq!(side(&f["favicon/android-chrome-512x512.png"]), 512);
    let manifest: serde_json::Value =
        serde_json::from_slice(&f["favicon/site.webmanifest"]).unwrap();
    assert_eq!(manifest["name"], "logo");
    assert_eq!(manifest["icons"][1]["sizes"], "512x512");
    let head = String::from_utf8(f["favicon/head.html"].clone()).unwrap();
    assert!(head.contains(r#"<link rel="manifest" href="/site.webmanifest">"#));
}

#[test]
fn favicon_makes_only_chosen_files() {
    let f = run(json!({ "faviconApple": false, "faviconManifest": false }));
    let names: Vec<_> = f.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "favicon/favicon-16x16.png",
            "favicon/favicon-32x32.png",
            "favicon/favicon.ico",
            "favicon/head.html",
        ]
    );
    let head = String::from_utf8(f["favicon/head.html"].clone()).unwrap();
    assert!(head.contains("/favicon.ico"));
    assert!(!head.contains("apple-touch-icon"));
    assert!(!head.contains("manifest"));
}

#[test]
fn favicon_needs_at_least_one_file() {
    let r = run_on(
        photo(64, 64),
        json!({ "faviconIco": false, "faviconPng": false, "faviconApple": false, "faviconManifest": false }),
    );
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
}

#[test]
fn android_set_matches_spec() {
    let f =
        run(json!({ "favicon": false, "android": true, "fill": true, "background": "#123456" }));
    for (density, n) in [
        ("mdpi", 48),
        ("hdpi", 72),
        ("xhdpi", 96),
        ("xxhdpi", 144),
        ("xxxhdpi", 192),
    ] {
        let dir = format!("android/res/mipmap-{density}");
        assert_eq!(side(&f[&format!("{dir}/ic_launcher.png")]), n);
        assert_eq!(side(&f[&format!("{dir}/ic_launcher_round.png")]), n);
        assert_eq!(
            side(&f[&format!("{dir}/ic_launcher_foreground.png")]),
            n * 108 / 48
        );
    }
    // 둥근 아이콘은 모서리가 투명, 적응형 전경은 안전 영역 밖이 투명
    assert_eq!(
        png(&f["android/res/mipmap-xxxhdpi/ic_launcher_round.png"])
            .get_pixel(0, 0)
            .0[3],
        0
    );
    let fg = png(&f["android/res/mipmap-xxxhdpi/ic_launcher_foreground.png"]);
    assert_eq!(fg.get_pixel(40, 216).0[3], 0);
    assert_eq!(fg.get_pixel(216, 216).0[3], 255);
    let xml =
        String::from_utf8(f["android/res/mipmap-anydpi-v26/ic_launcher.xml"].clone()).unwrap();
    assert!(xml.contains("@mipmap/ic_launcher_foreground"));
    let color =
        String::from_utf8(f["android/res/values/ic_launcher_background.xml"].clone()).unwrap();
    assert!(color.contains(">#123456<"));
    assert_eq!(side(&f["android/play-store-512.png"]), 512);
}

#[test]
fn ios_set_is_complete_and_opaque() {
    let f = run(json!({ "favicon": false, "ios": true, "radius": 30 }));
    let contents: serde_json::Value =
        serde_json::from_slice(&f["ios/AppIcon.appiconset/Contents.json"]).unwrap();
    let images = contents["images"].as_array().unwrap();
    assert_eq!(images.len(), 18);
    for i in images {
        let file = format!("ios/AppIcon.appiconset/{}", i["filename"].as_str().unwrap());
        let pt: f32 = i["size"]
            .as_str()
            .unwrap()
            .split('x')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let scale: f32 = i["scale"]
            .as_str()
            .unwrap()
            .trim_end_matches('x')
            .parse()
            .unwrap();
        assert_eq!(side(&f[&file]), (pt * scale).round() as u32, "{file}");
    }
    assert!(
        images
            .iter()
            .any(|i| i["size"] == "83.5x83.5" && i["filename"] == "Icon-167.png")
    );
    // App Store 1024는 투명 없음 (둥근 모서리도 OS가 깎으므로 적용 안 함)
    let store = png(&f["ios/AppIcon.appiconset/Icon-1024.png"]);
    assert_eq!(store.dimensions(), (1024, 1024));
    assert!(store.pixels().all(|p| p.0[3] == 255));
}

#[test]
fn windows_and_macos_files() {
    let f = run(json!({ "favicon": false, "windows": true, "macos": true }));
    assert_eq!(
        ico_sizes(&f["windows/app.ico"]),
        [16, 24, 32, 48, 64, 128, 256]
    );
    let icns = &f["macos/app.icns"];
    assert_eq!(&icns[..4], b"icns");
    assert_eq!(
        u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
        icns.len()
    );
    let mut at = 8;
    let mut codes = vec![];
    while at < icns.len() {
        let len = u32::from_be_bytes(icns[at + 4..at + 8].try_into().unwrap()) as usize;
        codes.push(String::from_utf8(icns[at..at + 4].to_vec()).unwrap());
        assert!(icns[at + 8..at + len].starts_with(b"\x89PNG"));
        at += len;
    }
    assert_eq!(at, icns.len());
    assert!(codes.contains(&"ic10".to_string()), "1024(512@2x) 포함");
    assert_eq!(codes.len(), 11);
}

#[test]
fn padding_radius_and_background() {
    let img = image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        400,
        400,
        image::Rgba([200, 0, 0, 255]),
    ));
    let f = run_on(img.clone(), json!({ "radius": 50 })).unwrap();
    let round = png(&f["favicon/android-chrome-512x512.png"]);
    assert_eq!(round.get_pixel(0, 0).0[3], 0, "50%면 원: 모서리 투명");
    assert_eq!(round.get_pixel(256, 256).0, [200, 0, 0, 255]);

    let f = run_on(img.clone(), json!({ "padding": 25 })).unwrap();
    let padded = png(&f["favicon/android-chrome-512x512.png"]);
    assert_eq!(padded.get_pixel(64, 256).0[3], 0, "여백은 투명");
    assert_eq!(padded.get_pixel(256, 256).0[3], 255);

    let f = run_on(
        img,
        json!({ "padding": 25, "fill": true, "background": "#00ff00" }),
    )
    .unwrap();
    assert_eq!(
        png(&f["favicon/android-chrome-512x512.png"])
            .get_pixel(64, 256)
            .0,
        [0, 255, 0, 255]
    );
}

#[test]
fn wide_image_is_centered_not_stretched() {
    let img = image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        800,
        200,
        image::Rgba([0, 0, 255, 255]),
    ));
    let f = run_on(img, json!({})).unwrap();
    let out = png(&f["favicon/android-chrome-512x512.png"]);
    assert_eq!(out.get_pixel(256, 20).0[3], 0, "위아래는 투명");
    assert_eq!(out.get_pixel(256, 256).0, [0, 0, 255, 255]);
}

#[test]
fn needs_a_target_and_valid_color() {
    let r = run_on(photo(64, 64), json!({ "favicon": false }));
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
    let r = run_on(photo(64, 64), json!({ "fill": true, "background": "빨강" }));
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
}

#[test]
fn first_file_is_large_preview() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("logo.png");
    photo(300, 300).save(&src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool()
        .run(&ctx, &[src], dir.path(), &json!({ "ios": true }))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        names[0],
        PathBuf::from("logo-icons/ios/AppIcon.appiconset/Icon-1024.png")
    );
    assert!(Path::new(&dir.path().join(&names[0])).is_file());
}
