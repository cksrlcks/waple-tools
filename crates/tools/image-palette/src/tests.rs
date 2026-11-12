use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use serde_json::{Value, json};
use waple_tools_core::{Block, JobContext, ToolError, ToolOutput};
use waple_tools_image_common::testkit::{alpha_png, animated_gif, photo};

use super::*;
use crate::color::{BLACK, WHITE, from_hsl};

const RED: [u8; 3] = [0xe0, 0x10, 0x10];
const SLATE: [u8; 3] = [0x2a, 0x2e, 0x33];

fn run_blocks(path: &Path, opts: Value) -> Result<Vec<Block>, ToolError> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    match tool().run(&ctx, &[path.to_path_buf()], out.path(), &opts)? {
        ToolOutput::Blocks(b) => Ok(b),
        ToolOutput::Files(_) => panic!("블록이어야 함"),
    }
}

/// (hex, label, text)
fn colors(blocks: &[Block]) -> Vec<(String, String, Option<String>)> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Color {
                hex, label, text, ..
            } => Some((hex.clone(), label.clone().unwrap(), text.clone())),
            _ => None,
        })
        .collect()
}

fn code(blocks: &[Block], name: &str) -> String {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Code { code, name: n, .. } if n.as_deref() == Some(name) => Some(code.clone()),
            _ => None,
        })
        .unwrap()
}

/// 왼쪽 3/4는 a, 오른쪽 1/4는 b
fn two_regions(dir: &Path, w: u32, h: u32, a: [u8; 3], b: [u8; 3]) -> PathBuf {
    let p = dir.join("two.png");
    RgbaImage::from_fn(w, h, |x, _| {
        let c = if x < w * 3 / 4 { a } else { b };
        Rgba([c[0], c[1], c[2], 255])
    })
    .save(&p)
    .unwrap();
    p
}

fn photo_png(dir: &Path) -> PathBuf {
    let p = dir.join("photo.png");
    photo(300, 200).save(&p).unwrap();
    p
}

#[test]
fn two_colors_and_ratio_after_downscale() {
    let dir = tempfile::tempdir().unwrap();
    // 긴 변 800px → 200px로 줄여 계산해도 비율은 그대로
    let p = two_regions(dir.path(), 800, 200, RED, SLATE);
    let b = run_blocks(&p, json!({})).unwrap();
    let c = colors(&b);
    assert_eq!(c.len(), 2, "서로 다른 색이 둘뿐이면 둘만");
    assert_eq!((c[0].0.as_str(), c[0].1.as_str()), ("#e01010", "75.0%"));
    assert_eq!((c[1].0.as_str(), c[1].1.as_str()), ("#2a2e33", "25.0%"));
    assert!(c.iter().all(|x| x.2.is_none()), "주요 색상엔 글자색 없음");
}

#[test]
fn count_limits_colors() {
    let dir = tempfile::tempdir().unwrap();
    let p = photo_png(dir.path());
    for n in [3, 6, 12] {
        let b = run_blocks(&p, json!({ "count": n })).unwrap();
        assert_eq!(colors(&b).len(), n);
    }
    let err = run_blocks(&p, json!({ "count": 2 })).unwrap_err();
    assert!(matches!(err, ToolError::InvalidOptions(_)));
}

fn rgb_of(hex: &str) -> [u8; 3] {
    let v = u32::from_str_radix(&hex[1..], 16).unwrap();
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

#[test]
fn sorts_by_ratio_or_brightness() {
    let dir = tempfile::tempdir().unwrap();
    let p = photo_png(dir.path());
    let by_ratio = colors(&run_blocks(&p, json!({ "count": 8 })).unwrap());
    let pcts: Vec<f64> = by_ratio
        .iter()
        .map(|c| c.1.trim_end_matches('%').parse().unwrap())
        .collect();
    assert!(pcts.windows(2).all(|w| w[0] >= w[1]), "{pcts:?}");

    let by_light = colors(&run_blocks(&p, json!({ "count": 8, "sort": "brightness" })).unwrap());
    let lum: Vec<f64> = by_light.iter().map(|c| luminance(rgb_of(&c.0))).collect();
    assert!(lum.windows(2).all(|w| w[0] <= w[1]), "어두운 색부터");
    // 순서만 다르고 같은 색
    let mut a: Vec<_> = by_ratio.iter().map(|c| &c.0).collect();
    let mut b: Vec<_> = by_light.iter().map(|c| &c.0).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn same_input_same_output() {
    let dir = tempfile::tempdir().unwrap();
    let p = photo_png(dir.path());
    for opts in [json!({ "count": 12 }), json!({ "mode": "roles" })] {
        let a = serde_json::to_value(run_blocks(&p, opts.clone()).unwrap()).unwrap();
        let b = serde_json::to_value(run_blocks(&p, opts).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}

#[test]
fn transparent_pixels_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("alpha.png");
    alpha_png(&p, 40, 20);
    let c = colors(&run_blocks(&p, json!({})).unwrap());
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].0.as_str(), c[0].1.as_str()), ("#0000ff", "100.0%"));

    let empty = dir.path().join("empty.png");
    RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 10]))
        .save(&empty)
        .unwrap();
    for mode in ["dominant", "roles"] {
        let b = run_blocks(&empty, json!({ "mode": mode })).unwrap();
        assert!(matches!(b.as_slice(), [Block::Text { .. }]), "안내 글만");
    }
}

#[test]
fn roles_pick_vibrant_and_dark_muted() {
    let dir = tempfile::tempdir().unwrap();
    let p = two_regions(dir.path(), 40, 10, RED, SLATE);
    let b = run_blocks(&p, json!({ "mode": "roles" })).unwrap();
    let c = colors(&b);
    let names: Vec<_> = c.iter().map(|x| x.1.split(" · ").next().unwrap()).collect();
    assert_eq!(names, ROLES, "빠진 역할은 다른 역할에서 만든다");
    assert_eq!(
        (c[0].0.as_str(), c[0].1.as_str()),
        ("#e01010", "Vibrant · 75.0%")
    );
    assert_eq!(
        (c[4].0.as_str(), c[4].1.as_str()),
        ("#2a2e33", "DarkMuted · 25.0%")
    );
    assert_eq!(c[1].1, "DarkVibrant · 만든 색");
    assert!(!b.iter().any(|x| matches!(x, Block::Text { .. })));
    for (hex, _, text) in &c {
        assert_eq!(rgb_of(text.as_deref().unwrap()), body_text(rgb_of(hex)));
    }
}

#[test]
fn missing_roles_are_left_out_with_note() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("gray.png");
    RgbaImage::from_pixel(10, 10, Rgba([128, 128, 128, 255]))
        .save(&p)
        .unwrap();
    let b = run_blocks(&p, json!({ "mode": "roles" })).unwrap();
    let c = colors(&b);
    assert_eq!(c.len(), 1);
    assert!(c[0].1.starts_with("Muted"));
    let Block::Text { text } = &b[0] else {
        panic!("안내 글이 먼저");
    };
    assert!(
        text.contains("Vibrant") && text.contains("LightMuted"),
        "{text}"
    );
}

#[test]
fn body_text_uses_yiq_150() {
    // YIQ = (r*299 + g*587 + b*114) / 1000. 회색 v의 YIQ는 v
    assert_eq!(body_text([149; 3]), WHITE);
    assert_eq!(body_text([150; 3]), BLACK);
    assert_eq!(body_text([255, 255, 0]), BLACK);
    assert_eq!(body_text([0, 0, 128]), WHITE);
    assert_eq!(body_text([255, 255, 255]), BLACK);
}

#[test]
fn hsl_round_trips() {
    for c in [RED, SLATE, [12, 200, 99], [255, 255, 255], [0, 0, 0]] {
        assert_eq!(from_hsl(to_hsl(c)), c);
    }
}

#[test]
fn css_and_json_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let p = two_regions(dir.path(), 40, 10, RED, SLATE);
    let b = run_blocks(&p, json!({})).unwrap();
    assert_eq!(
        code(&b, "palette.css"),
        ":root {\n  --palette-1: #e01010;\n  --palette-2: #2a2e33;\n}\n"
    );
    let j: Value = serde_json::from_str(&code(&b, "palette.json")).unwrap();
    assert_eq!(
        j[0],
        json!({ "hex": "#e01010", "rgb": [224, 16, 16], "hsl": [0, 87, 47], "percent": 75.0 })
    );

    let b = run_blocks(&p, json!({ "mode": "roles" })).unwrap();
    let css = code(&b, "palette.css");
    assert!(
        css.starts_with(":root {\n  --vibrant: #e01010;\n  --vibrant-text: #ffffff;\n"),
        "{css}"
    );
    assert!(css.contains("  --dark-muted: #2a2e33;\n  --dark-muted-text: #ffffff;\n"));
    let j: Value = serde_json::from_str(&code(&b, "palette.json")).unwrap();
    assert_eq!(j.as_array().unwrap().len(), 6);
    assert_eq!(j[0]["role"], "Vibrant");
    assert_eq!(j[0]["bodyTextColor"], "#ffffff");
    assert_eq!(j[0]["derived"], false);
    assert_eq!(j[1]["derived"], true);
}

#[test]
fn not_an_image_fails() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("fake.png");
    std::fs::write(&p, "hello").unwrap();
    assert!(matches!(
        run_blocks(&p, json!({})),
        Err(ToolError::Failed(_))
    ));
}

#[test]
fn count_and_sort_only_in_dominant_mode() {
    let s = tool().options_schema();
    for key in ["count", "sort"] {
        assert_eq!(
            s["properties"][key]["x-enabled-when"],
            json!({ "mode": ["dominant"] })
        );
    }
}

#[test]
fn animated_gif_uses_first_frame() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("anim.gif");
    // 첫 프레임은 파란색 한 가지
    animated_gif(&p, 20, 20, 3);
    let blocks = run_blocks(&p, json!({})).unwrap();
    let c = colors(&blocks);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].0, "#0000ff");
}
