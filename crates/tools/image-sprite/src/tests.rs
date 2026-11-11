use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

const R: [u8; 4] = [255, 0, 0, 255];
const G: [u8; 4] = [0, 255, 0, 255];
const B: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

fn solid(dir: &Path, name: &str, w: u32, h: u32, c: [u8; 4]) -> PathBuf {
    let p = dir.join(name);
    RgbaImage::from_pixel(w, h, Rgba(c)).save(&p).unwrap();
    p
}

/// 실행 → `sprite/` 아래 파일 이름 → 내용. 원본이 그대로인지도 확인한다
fn run_on(
    inputs: &[PathBuf],
    opts: serde_json::Value,
) -> Result<BTreeMap<String, Vec<u8>>, ToolError> {
    let before: Vec<_> = inputs.iter().map(|p| std::fs::read(p).unwrap()).collect();
    let out = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, inputs, out.path(), &opts)? else {
        panic!("파일이어야 함");
    };
    for (p, b) in inputs.iter().zip(before) {
        assert_eq!(std::fs::read(p).unwrap(), b, "원본은 그대로");
    }
    Ok(names
        .iter()
        .map(|n| {
            let rel = n.strip_prefix("sprite").expect("sprite/ 아래");
            (
                rel.to_string_lossy().into_owned(),
                std::fs::read(out.path().join(n)).unwrap(),
            )
        })
        .collect())
}

fn png(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes).unwrap().to_rgba8()
}

fn px(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    img.get_pixel(x, y).0
}

fn css_of(files: &BTreeMap<String, Vec<u8>>) -> String {
    String::from_utf8(files["sprite.css"].clone()).unwrap()
}

fn rule(class: &str, w: u32, h: u32, pos: &str) -> String {
    format!(".{class} {{\n  width: {w}px;\n  height: {h}px;\n  background-position: {pos};\n}}")
}

#[test]
fn horizontal_places_left_to_right_with_gap() {
    let d = tempfile::tempdir().unwrap();
    let inputs = [
        solid(d.path(), "red.png", 3, 2, R),
        solid(d.path(), "blue.png", 4, 5, B),
    ];
    let files = run_on(&inputs, json!({ "layout": "horizontal", "gap": 1 })).unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["sprite.css", "sprite.png"]
    );
    let img = png(&files["sprite.png"]);
    assert_eq!(img.dimensions(), (8, 5));
    assert_eq!(px(&img, 0, 0), R);
    assert_eq!(px(&img, 2, 1), R);
    assert_eq!(px(&img, 3, 0), CLEAR, "간격");
    assert_eq!(px(&img, 0, 2), CLEAR, "투명 배경");
    assert_eq!(px(&img, 4, 0), B);
    assert_eq!(px(&img, 7, 4), B);
    let css = css_of(&files);
    assert!(css.contains(&rule("sprite-red", 3, 2, "0 0")), "{css}");
    assert!(css.contains(&rule("sprite-blue", 4, 5, "-4px 0")), "{css}");
    assert!(css.contains("background-image: url(sprite.png);"), "{css}");
    assert!(!css.contains("@2x"), "{css}");
}

#[test]
fn vertical_places_top_to_bottom_with_gap() {
    let d = tempfile::tempdir().unwrap();
    let inputs = [
        solid(d.path(), "red.png", 3, 2, R),
        solid(d.path(), "blue.png", 4, 5, B),
    ];
    let files = run_on(&inputs, json!({ "layout": "vertical", "gap": 3 })).unwrap();
    let img = png(&files["sprite.png"]);
    assert_eq!(img.dimensions(), (4, 10));
    assert_eq!(px(&img, 2, 1), R);
    assert_eq!(px(&img, 3, 0), CLEAR);
    assert_eq!(px(&img, 0, 4), CLEAR, "간격");
    assert_eq!(px(&img, 0, 5), B);
    assert_eq!(px(&img, 3, 9), B);
    assert!(css_of(&files).contains(&rule("sprite-blue", 4, 5, "0 -5px")));
}

#[test]
fn grid_is_near_square_with_largest_cell() {
    let d = tempfile::tempdir().unwrap();
    let inputs: Vec<_> = (0..5)
        .map(|i| {
            let (w, h) = if i == 2 { (3, 4) } else { (2, 2) };
            let c = if i == 4 { B } else { G };
            solid(d.path(), &format!("{i}.png"), w, h, c)
        })
        .collect();
    let files = run_on(&inputs, json!({ "layout": "grid", "gap": 0 })).unwrap();
    let img = png(&files["sprite.png"]);
    // 5장 → 3열 2행, 칸은 3×4
    assert_eq!(img.dimensions(), (9, 8));
    assert_eq!(px(&img, 6, 0), G, "3번째 칸");
    assert_eq!(px(&img, 8, 3), G);
    assert_eq!(px(&img, 2, 0), CLEAR, "작은 이미지는 칸 왼쪽 위");
    assert_eq!(px(&img, 3, 4), B, "5번째는 두 번째 줄 가운데");
    assert_eq!(px(&img, 5, 4), CLEAR);
    assert_eq!(px(&img, 6, 4), CLEAR, "빈 칸");
    let css = css_of(&files);
    // 숫자로 시작하는 파일 이름도 접두사가 앞에 붙어 올바른 클래스가 된다
    assert!(css.contains(&rule("sprite-4", 2, 2, "-3px -4px")), "{css}");
    assert!(css.contains(&rule("sprite-1", 2, 2, "-3px 0")), "{css}");
}

#[test]
fn default_grid_with_gap_and_single_input() {
    let d = tempfile::tempdir().unwrap();
    let one = [solid(d.path(), "logo.png", 5, 3, R)];
    let files = run_on(&one, json!({})).unwrap();
    assert_eq!(png(&files["sprite.png"]).dimensions(), (5, 3));
    assert!(css_of(&files).contains(&rule("sprite-logo", 5, 3, "0 0")));

    // 기본 간격 2px, 4장 → 2×2
    let four: Vec<_> = (0..4)
        .map(|i| solid(d.path(), &format!("i{i}.png"), 2, 2, G))
        .collect();
    let files = run_on(&four, json!({})).unwrap();
    let img = png(&files["sprite.png"]);
    assert_eq!(img.dimensions(), (6, 6));
    assert_eq!(px(&img, 4, 4), G);
    assert_eq!(px(&img, 2, 2), CLEAR);
}

#[test]
fn class_names_are_cleaned_and_deduplicated() {
    let stems = [
        "My Icon_1",
        "my icon 1",
        "한글 아이콘",
        "a(b)+c.v2",
        "()",
        "My-Icon-1",
        "  Tab__ON  ",
    ];
    let names = unique_names(&stems.map(String::from));
    assert_eq!(
        names,
        [
            "my-icon-1",
            "my-icon-1-2",
            "한글-아이콘",
            "abcv2",
            "image",
            "my-icon-1-3",
            "tab-on"
        ]
    );

    let d = tempfile::tempdir().unwrap();
    let inputs = [
        solid(d.path(), "한글 아이콘.png", 2, 2, R),
        solid(d.path(), "Home_Icon.png", 2, 2, G),
    ];
    let css = css_of(&run_on(&inputs, json!({ "prefix": " icon " })).unwrap());
    assert!(css.starts_with("@charset \"UTF-8\";\n"), "{css}");
    assert!(
        css.contains("<i class=\"icon icon-한글-아이콘\"></i>"),
        "{css}"
    );
    assert!(css.contains(".icon {\n  display: inline-block;\n  background-image: url(sprite.png);\n  background-repeat: no-repeat;\n}"), "{css}");
    assert!(css.contains(".icon-한글-아이콘 {"), "{css}");
    assert!(css.contains(".icon-home-icon {"), "{css}");
}

#[test]
fn bad_prefix_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    let inputs = [solid(d.path(), "a.png", 2, 2, R)];
    for p in ["", "1abc", "-a", "a b", "a.b"] {
        let err = run_on(&inputs, json!({ "prefix": p })).unwrap_err();
        assert!(matches!(err, ToolError::InvalidOptions(_)), "{p:?}: {err}");
    }
    assert!(run_on(&inputs, json!({ "prefix": "아이콘_x-1" })).is_ok());
}

#[test]
fn retina_makes_2x_and_half_with_even_cells() {
    let d = tempfile::tempdir().unwrap();
    // 홀수 크기 3×3은 2x 모드에서 4×4 칸이 된다
    let inputs = [
        solid(d.path(), "red.png", 3, 3, R),
        solid(d.path(), "blue.png", 4, 2, B),
    ];
    let files = run_on(
        &inputs,
        json!({ "layout": "horizontal", "gap": 1, "retina": true }),
    )
    .unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["sprite.css", "sprite.png", "sprite@2x.png"]
    );
    let full = png(&files["sprite@2x.png"]);
    assert_eq!(full.dimensions(), (10, 4), "4 + 간격 2 + 4");
    assert_eq!(px(&full, 0, 0), R);
    assert_eq!(px(&full, 2, 2), R);
    assert_eq!(px(&full, 3, 0), CLEAR, "짝수로 맞춘 칸의 빈 줄");
    assert_eq!(px(&full, 5, 0), CLEAR, "간격");
    assert_eq!(px(&full, 6, 0), B);
    assert_eq!(px(&full, 6, 2), CLEAR);

    let half = png(&files["sprite.png"]);
    assert_eq!(half.dimensions(), (5, 2));
    let [r, _, _, a] = px(&half, 0, 0);
    assert!(r > 200 && a > 200, "{:?}", px(&half, 0, 0));
    assert_eq!(px(&half, 2, 0), CLEAR, "간격 1px");
    let [_, _, b, a] = px(&half, 3, 0);
    assert!(b > 250 && a > 250, "{:?}", px(&half, 3, 0));
    assert_eq!(px(&half, 3, 1), CLEAR);

    let css = css_of(&files);
    assert!(css.contains(&rule("sprite-red", 2, 2, "0 0")), "{css}");
    assert!(css.contains(&rule("sprite-blue", 2, 1, "-3px 0")), "{css}");
    assert!(
        css.contains("  .sprite {\n    background-image: url(sprite@2x.png);\n    background-size: 5px 2px;\n  }"),
        "{css}"
    );
    assert!(css.contains("@media (-webkit-min-device-pixel-ratio: 2), (min-resolution: 192dpi) {"));
}

#[test]
fn non_image_fails_with_file_name() {
    let d = tempfile::tempdir().unwrap();
    let txt = d.path().join("note.png");
    std::fs::write(&txt, "그냥 글").unwrap();
    let inputs = [solid(d.path(), "a.png", 2, 2, R), txt];
    let err = run_on(&inputs, json!({})).unwrap_err();
    assert!(
        matches!(&err, ToolError::Failed(m) if m.starts_with("note.png:")),
        "{err}"
    );
}

#[test]
fn too_large_layout_fails() {
    let d = tempfile::tempdir().unwrap();
    // 넓이 합은 작아도 가로로 늘어놓으면 20001×6000
    let inputs = [
        solid(d.path(), "wide.png", 20_000, 1, R),
        solid(d.path(), "tall.png", 1, 6_000, B),
    ];
    let err = run_on(&inputs, json!({ "layout": "horizontal" })).unwrap_err();
    assert!(matches!(err, ToolError::Failed(m) if m.contains("너무 큽니다")));
}

/// 픽셀 데이터 없이 크기만 적힌 24비트 BMP 머리글 (디코딩하면 실패한다)
fn bmp_header_only(dir: &Path, name: &str, w: i32, h: i32) -> PathBuf {
    let mut b = Vec::new();
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&54u32.to_le_bytes()); // 파일 크기
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&54u32.to_le_bytes()); // 픽셀 시작 위치
    b.extend_from_slice(&40u32.to_le_bytes()); // BITMAPINFOHEADER
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    let p = dir.join(name);
    std::fs::write(&p, b).unwrap();
    p
}

#[test]
fn too_large_result_fails_before_allocating() {
    let d = tempfile::tempdir().unwrap();
    // 한 장씩은 상한 아래(5600만)지만 합이 1억을 넘는다. 디코딩까지 갔다면 "읽을 수 없습니다"로 실패했을 것
    let inputs = [
        bmp_header_only(d.path(), "a.bmp", 8000, 7000),
        bmp_header_only(d.path(), "b.bmp", 8000, 7000),
    ];
    let err = run_on(&inputs, json!({})).unwrap_err();
    assert!(
        matches!(&err, ToolError::Failed(m) if m.contains("너무 큽니다")),
        "{err}"
    );
}

#[test]
fn retina_half_keeps_edge_color() {
    let d = tempfile::tempdir().unwrap();
    // 투명 배경 위 흰 띠 3px: 1x로 줄이면 가장자리가 반투명이 된다
    let icon = d.path().join("icon.png");
    RgbaImage::from_fn(8, 8, |x, _| {
        if (2..5).contains(&x) {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba(CLEAR)
        }
    })
    .save(&icon)
    .unwrap();
    let files = run_on(&[icon], json!({ "retina": true })).unwrap();
    let half = png(&files["sprite.png"]);
    let edges: Vec<_> = half
        .pixels()
        .filter(|p| (32..255).contains(&p[3]))
        .collect();
    assert!(!edges.is_empty(), "반투명 가장자리가 있어야 함");
    for p in edges {
        assert!(p.0[..3].iter().all(|&c| c > 240), "어두워지면 안 됨: {p:?}");
    }
}

#[test]
fn schema_has_no_preview() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "none");
}
