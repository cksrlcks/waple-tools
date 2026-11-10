//! 여백: 자동 자르기(트림), 여백 추가, 정사각형 캔버스

use image::{DynamicImage, Rgba, RgbaImage};
use localtools_core::ToolError;
use localtools_image_common::ensure_pixels;

/// 투명 픽셀이 있으면 알파가 기준 이하인 바깥 테두리를, 없으면 왼쪽 위 픽셀 색과 비슷한 테두리를 잘라낸다.
/// `tolerance`는 0~100 (%). 남는 픽셀이 없으면 원본 그대로.
pub(crate) fn trim(img: DynamicImage, tolerance: u8) -> DynamicImage {
    let rgba = img.to_rgba8();
    let limit = (u16::from(tolerance.min(100)) * 255 / 100) as u8;
    let transparent = rgba.pixels().any(|p| p.0[3] < 255);
    let corner = rgba.get_pixel(0, 0).0;
    let is_background = |p: &Rgba<u8>| {
        if transparent {
            p.0[3] <= limit
        } else {
            (0..3).all(|i| p.0[i].abs_diff(corner[i]) <= limit)
        }
    };
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in rgba.enumerate_pixels() {
        if !is_background(p) {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 == u32::MAX {
        return img;
    }
    img.crop_imm(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
}

/// `w`×`h` 캔버스(채움색) 가운데에 이미지를 놓는다
fn place(img: &DynamicImage, w: u32, h: u32, fill: Rgba<u8>) -> DynamicImage {
    let mut canvas = RgbaImage::from_pixel(w, h, fill);
    let x = i64::from((w - img.width()) / 2);
    let y = i64::from((h - img.height()) / 2);
    image::imageops::overlay(&mut canvas, &img.to_rgba8(), x, y);
    DynamicImage::ImageRgba8(canvas)
}

/// 네 변에 `px`만큼 여백을 더한다
pub(crate) fn pad(img: DynamicImage, px: u32, fill: Rgba<u8>) -> Result<DynamicImage, ToolError> {
    let px = px.min(10_000);
    let (w, h) = (img.width() + px * 2, img.height() + px * 2);
    ensure_pixels(w, h)?;
    Ok(place(&img, w, h, fill))
}

/// 짧은 변을 늘려 정사각형 캔버스에 가운데 놓는다
pub(crate) fn square(img: DynamicImage, fill: Rgba<u8>) -> Result<DynamicImage, ToolError> {
    let side = img.width().max(img.height());
    ensure_pixels(side, side)?;
    Ok(place(&img, side, side, fill))
}
