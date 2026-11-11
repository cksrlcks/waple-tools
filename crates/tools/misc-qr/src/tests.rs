use serde_json::json;
use waple_tools_core::{Block, JobContext, ToolError, ToolOutput};

use super::*;

fn run(opts: serde_json::Value) -> Result<Vec<Block>, ToolError> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    match tool().run(&ctx, &[], out.path(), &opts)? {
        ToolOutput::Blocks(b) => {
            assert_eq!(
                std::fs::read_dir(out.path()).unwrap().count(),
                0,
                "파일을 만들지 않는다"
            );
            Ok(b)
        }
        ToolOutput::Files(_) => panic!("블록이어야 함"),
    }
}

/// 이미지 블록의 PNG → (한 변, RGB 픽셀)
fn png_of(blocks: &[Block]) -> (u32, Vec<u8>) {
    let src = blocks
        .iter()
        .find_map(|b| match b {
            Block::Image { src, name } => {
                assert_eq!(name, "qr.png");
                Some(src.clone())
            }
            _ => None,
        })
        .unwrap();
    let b64 = src.strip_prefix("data:image/png;base64,").unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.width, info.height);
    assert_eq!(info.color_type, png::ColorType::Rgb);
    (info.width, buf)
}

fn svg_of(blocks: &[Block]) -> &str {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Code { lang, code, name } => {
                assert_eq!((lang.as_str(), name.as_deref()), ("svg", Some("qr.svg")));
                Some(code.as_str())
            }
            _ => None,
        })
        .unwrap()
}

fn px(buf: &[u8], side: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * side + x) * 3) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

#[test]
fn empty_url_asks_for_input() {
    for url in ["", "   "] {
        let b = run(json!({ "url": url })).unwrap();
        assert!(matches!(&b[..], [Block::Text { text }] if text.contains("URL을 입력")));
    }
    assert!(matches!(&run(json!({})).unwrap()[..], [Block::Text { .. }]));
}

#[test]
fn png_has_exact_cells_margin_and_colors() {
    let b = run(json!({ "url": "https://example.com", "size": 512 })).unwrap();
    assert!(
        !b.iter().any(|b| matches!(b, Block::Text { .. })),
        "기본 색은 경고 없음"
    );
    let (side, buf) = png_of(&b);
    // 버전 2(25칸) + 여백 4칸×2 = 33칸 → 칸당 15px
    assert_eq!(side, 33 * 15);
    assert!(matches!(
        b.iter().find(|b| matches!(b, Block::Number { .. })),
        Some(Block::Number { value, .. }) if *value == side as f64
    ));
    assert_eq!(px(&buf, side, 0, 0), [255, 255, 255], "여백은 배경색");
    assert_eq!(px(&buf, side, 4 * 15 - 1, 4 * 15 - 1), [255, 255, 255]);
    assert_eq!(
        px(&buf, side, 4 * 15, 4 * 15),
        [0, 0, 0],
        "왼쪽 위 찾기 무늬"
    );
}

#[test]
fn margin_and_colors_follow_options() {
    let b = run(json!({
        "url": "https://example.com",
        "margin": 0,
        "foreground": "#123456",
        "background": "#fafafa",
    }))
    .unwrap();
    let (side, buf) = png_of(&b);
    assert_eq!(side % 25, 0);
    assert_eq!(
        px(&buf, side, 0, 0),
        [0x12, 0x34, 0x56],
        "여백 0이면 바로 찾기 무늬"
    );
    let svg = svg_of(&b);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 25 25\""));
    assert!(svg.contains(&format!("width=\"{side}\"")));
    assert!(svg.contains("fill=\"#fafafa\"") && svg.contains("fill=\"#123456\""));
    assert!(
        svg.contains("d=\"M0 0h7v1h-7z"),
        "찾기 무늬 윗줄 7칸이 한 사각형"
    );
}

#[test]
fn higher_error_correction_makes_denser_code() {
    let n = |ec: &str| {
        png_of(
            &run(json!({ "url": "https://example.com/abc", "errorCorrection": ec, "size": 2048 }))
                .unwrap(),
        )
        .0
    };
    // 23바이트: 낮음은 버전 2(25칸), 최고는 버전 3(29칸) → 여백 포함 33칸×62px, 37칸×55px
    assert_eq!((n("low"), n("high")), (33 * 62, 37 * 55));
}

#[test]
fn tiny_size_still_draws_one_px_cells() {
    let long = format!("https://example.com/{}", "a".repeat(1500));
    let (side, _) = png_of(&run(json!({ "url": long, "size": 128 })).unwrap());
    assert!(side > 128, "칸이 128개를 넘으면 칸당 1px");
}

#[test]
fn too_long_fails_with_message() {
    let r = run(json!({ "url": "a".repeat(5000) }));
    assert!(matches!(r, Err(ToolError::Failed(m)) if m.contains("너무 깁니다")));
}

#[test]
fn bad_color_is_invalid_and_never_reaches_svg() {
    let r = run(json!({ "url": "a", "foreground": "red\"/><script>" }));
    assert!(matches!(r, Err(ToolError::InvalidOptions(m)) if m.contains("전경색")));
    assert!(matches!(
        run(json!({ "url": "a", "errorCorrection": "nope" })),
        Err(ToolError::InvalidOptions(_))
    ));
}

#[test]
fn low_contrast_warns() {
    let b = run(json!({ "url": "a", "foreground": "#ffffff", "background": "#000000" })).unwrap();
    assert!(matches!(&b[0], Block::Text { text } if text.contains("어둡지 않습니다")));
    let b = run(json!({ "url": "a", "foreground": "#777777", "background": "#888888" })).unwrap();
    assert!(matches!(&b[0], Block::Text { .. }));
}

#[test]
fn schema_has_no_input_flag() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["properties"]["url"]["x-wide"], true);
}
