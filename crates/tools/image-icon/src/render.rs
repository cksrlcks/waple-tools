//! 원본 → 정사각형 원판 → 크기·모양별 아이콘 한 장

use image::imageops::{self, FilterType};
use image::{DynamicImage, Rgba, RgbaImage};

/// 크기마다 다시 그릴 원판의 긴 변 상한 (1024px 아이콘까지 충분)
const MASTER_MAX: u32 = 1024;

/// 모든 크기의 출발점: 비율을 지켜 정사각형 투명 캔버스 가운데에 둔다
pub fn square_master(img: &DynamicImage) -> RgbaImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let side = w.max(h).max(1);
    let scale = (f64::from(MASTER_MAX) / f64::from(side)).min(1.0);
    let (w2, h2) = (
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
    );
    let fitted = if scale < 1.0 {
        imageops::resize(&rgba, w2, h2, FilterType::Lanczos3)
    } else {
        rgba
    };
    let side = w2.max(h2);
    let mut canvas = RgbaImage::new(side, side);
    imageops::overlay(
        &mut canvas,
        &fitted,
        i64::from((side - w2) / 2),
        i64::from((side - h2) / 2),
    );
    canvas
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mask {
    None,
    /// 모서리 반지름 = 한 변 × 비율 (0~0.5)
    Rounded(f32),
    Circle,
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// 한 변에 대한 여백 비율 (0~0.4)
    pub padding: f32,
    /// 내용이 차지할 수 있는 비율 (적응형 아이콘 전경: 66/108)
    pub safe: f32,
    pub mask: Mask,
    /// 배경색. None이면 투명
    pub background: Option<[u8; 3]>,
}

/// n×n 아이콘 한 장
pub fn render(master: &RgbaImage, n: u32, s: Style) -> RgbaImage {
    let content = ((n as f32) * s.safe * (1.0 - 2.0 * s.padding))
        .round()
        .max(1.0) as u32;
    let scaled = imageops::resize(master, content, content, FilterType::Lanczos3);
    let mut out = match s.background {
        Some([r, g, b]) => RgbaImage::from_pixel(n, n, Rgba([r, g, b, 255])),
        None => RgbaImage::new(n, n),
    };
    let off = i64::from((n - content.min(n)) / 2);
    imageops::overlay(&mut out, &scaled, off, off);
    apply_mask(&mut out, s.mask);
    out
}

/// 가장자리를 부드럽게 깎는다 (모서리까지의 거리로 알파를 정한다)
fn apply_mask(img: &mut RgbaImage, mask: Mask) {
    let n = img.width() as f32;
    let r = match mask {
        Mask::None => return,
        Mask::Rounded(k) if k <= 0.0 => return,
        Mask::Rounded(k) => n * k.min(0.5),
        Mask::Circle => n / 2.0,
    };
    let half = n / 2.0;
    for (x, y, p) in img.enumerate_pixels_mut() {
        // 둥근 사각형까지의 부호 있는 거리 (안쪽이 음수)
        let qx = ((x as f32 + 0.5) - half).abs() - (half - r);
        let qy = ((y as f32 + 0.5) - half).abs() - (half - r);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        let d = outside + qx.max(qy).min(0.0) - r;
        let cover = (0.5 - d).clamp(0.0, 1.0);
        p.0[3] = (f32::from(p.0[3]) * cover).round() as u8;
    }
}
