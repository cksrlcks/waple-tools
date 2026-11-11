//! 투명 이미지 크기 바꾸기

use image::RgbaImage;
use image::imageops::{FilterType, resize};

/// 알파를 곱한 채로 크기를 바꾼다. 그냥 줄이면 투명 픽셀의 색(보통 검정)이 반투명 가장자리로 번져 어두워진다.
pub fn resize_rgba(img: &RgbaImage, w: u32, h: u32, filter: FilterType) -> RgbaImage {
    let mut pm = img.clone();
    for p in pm.pixels_mut() {
        let a = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    let mut out = resize(&pm, w, h, filter);
    for p in out.pixels_mut() {
        let a = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = (u16::from(*c) * 255 + a / 2)
                .checked_div(a)
                .map_or(0, |v| v.min(255) as u8);
        }
    }
    out
}
