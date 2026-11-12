//! 원본 위에 영역 상자와 읽기 순서 번호를 그린 작은 PNG (`data:` URI)

use base64::Engine;
use image::{Rgb, RgbImage};
use waple_tools_core::ToolError;

use crate::layout::{Line, Rect};
use crate::regions::Region;

/// 결과 블록 안에서 보는 크기 (블록은 DLL 경계를 JSON으로 넘으니 작게)
const MAX_W: u32 = 720;
const MAX_H: u32 = 432;
const COLORS: [[u8; 3]; 6] = [
    [37, 99, 235],
    [220, 38, 38],
    [22, 163, 74],
    [217, 119, 6],
    [147, 51, 234],
    [8, 145, 178],
];
/// 3×5 숫자 글꼴 (윗줄부터, 줄마다 3비트)
const DIGITS: [u16; 10] = [
    0o75557, 0o26227, 0o71747, 0o71717, 0o55711, 0o74717, 0o74757, 0o71111, 0o75757, 0o75717,
];
const DOT: u32 = 3;

pub fn render(img: &RgbImage, lines: &[Line], regions: &[Region]) -> Result<String, ToolError> {
    let s = (MAX_W as f32 / img.width() as f32)
        .min(MAX_H as f32 / img.height() as f32)
        .min(1.0);
    let (w, h) = (
        ((img.width() as f32 * s).round() as u32).max(1),
        ((img.height() as f32 * s).round() as u32).max(1),
    );
    let mut out = image::imageops::resize(img, w, h, image::imageops::FilterType::Triangle);
    for (n, r) in regions.iter().enumerate() {
        let Some(area) = r
            .lines
            .iter()
            .map(|&i| lines[i].rect)
            .reduce(|a, b| a.union(&b))
        else {
            continue;
        };
        let c = Rgb(COLORS[n % COLORS.len()]);
        let scaled = Rect {
            x0: area.x0 * s - 2.0,
            y0: area.y0 * s - 2.0,
            x1: area.x1 * s + 2.0,
            y1: area.y1 * s + 2.0,
        };
        outline(&mut out, &scaled, c);
        number(&mut out, &scaled, n + 1, c);
    }
    let mut png = Vec::new();
    out.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| ToolError::Failed(format!("영역 이미지를 만들 수 없습니다: {e}")))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

fn fill(img: &mut RgbImage, x0: i64, y0: i64, x1: i64, y1: i64, c: Rgb<u8>) {
    let (w, h) = (i64::from(img.width()), i64::from(img.height()));
    for y in y0.max(0)..y1.min(h) {
        for x in x0.max(0)..x1.min(w) {
            img.put_pixel(x as u32, y as u32, c);
        }
    }
}

fn outline(img: &mut RgbImage, r: &Rect, c: Rgb<u8>) {
    let (x0, y0, x1, y1) = (r.x0 as i64, r.y0 as i64, r.x1 as i64, r.y1 as i64);
    fill(img, x0, y0, x1, y0 + 2, c);
    fill(img, x0, y1 - 2, x1, y1, c);
    fill(img, x0, y0, x0 + 2, y1, c);
    fill(img, x1 - 2, y0, x1, y1, c);
}

/// 색 바탕 흰 번호: 상자 왼쪽 위 바깥 → 자리가 없으면 오른쪽 바깥 → 그래도 없으면 안쪽 (글자를 덮지 않게)
fn number(img: &mut RgbImage, r: &Rect, n: usize, c: Rgb<u8>) {
    let s = n.to_string();
    let (cw, ch) = (4 * DOT as i64, 5 * DOT as i64);
    let bw = cw * s.len() as i64 + 2 * DOT as i64;
    let bh = ch + 4 * DOT as i64 / 2 + 2;
    let (x, y, right) = (r.x0 as i64, r.y0 as i64, r.x1 as i64);
    let (x0, y0) = if y - bh >= 0 {
        (x, y - bh)
    } else if right + bw <= i64::from(img.width()) {
        (right, y)
    } else {
        (x, y)
    };
    fill(img, x0, y0, x0 + bw, y0 + bh, c);
    let white = Rgb([255, 255, 255]);
    for (k, d) in s.bytes().enumerate() {
        let g = DIGITS[(d - b'0') as usize];
        for row in 0..5 {
            for col in 0..3 {
                if g >> ((4 - row) * 3 + (2 - col)) & 1 == 1 {
                    let px = x0 + DOT as i64 * 2 + k as i64 * cw + col * DOT as i64;
                    let py = y0 + DOT as i64 + 1 + row * DOT as i64;
                    fill(img, px, py, px + DOT as i64, py + DOT as i64, white);
                }
            }
        }
    }
}
