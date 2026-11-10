//! 크기 단계: 결과 크기 계산과 리사이즈 (옛 리사이즈 도구에서 옮기고 SNS 규격·여백 추가를 더함)

use image::imageops::FilterType;
use image::{DynamicImage, Rgba, RgbaImage};
use waple_tools_core::ToolError;
use waple_tools_image_common::ensure_pixels;

use crate::{Fit, OptimizeOptions, Preset, SizeMode};

/// 한 변의 최대 크기
const MAX_SIDE: u32 = 20_000;

impl Preset {
    fn size(self) -> (u32, u32) {
        match self {
            Preset::Og => (1200, 630),
            Preset::X => (1600, 900),
            Preset::InstaSquare => (1080, 1080),
            Preset::InstaPortrait => (1080, 1350),
            Preset::Story => (1080, 1920),
        }
    }
}

/// 비율 유지 환산: `n * to / from` (최소 1)
fn scale(n: u32, from: u32, to: u32) -> u32 {
    ((f64::from(n) * f64::from(to) / f64::from(from.max(1))).round() as u32).max(1)
}

/// 상자 안에 비율 유지로 들어가는 크기
fn contain(w: u32, h: u32, bw: u32, bh: u32) -> (u32, u32) {
    let r = (f64::from(bw) / f64::from(w)).min(f64::from(bh) / f64::from(h));
    (
        ((f64::from(w) * r).round() as u32).max(1),
        ((f64::from(h) * r).round() as u32).max(1),
    )
}

/// 정확한 크기·SNS 규격일 때의 상자
fn target_box(o: &OptimizeOptions) -> Option<(u32, u32)> {
    match o.size {
        SizeMode::Exact => Some((o.width.clamp(1, MAX_SIDE), o.height.clamp(1, MAX_SIDE))),
        SizeMode::Preset => Some(o.preset.size()),
        _ => None,
    }
}

/// 결과 크기. 크기 단계가 꺼져 있으면 원본. 확대 금지면 어느 한 변이라도 커질 때 원본 크기
/// (여백 추가는 캔버스가 상자 크기이고, 안의 이미지만 확대하지 않는다).
pub(crate) fn target_size(w: u32, h: u32, o: &OptimizeOptions) -> (u32, u32) {
    if !o.resize {
        return (w, h);
    }
    let bw = o.width.clamp(1, MAX_SIDE);
    let bh = o.height.clamp(1, MAX_SIDE);
    let (tw, th) = match (o.size, target_box(o)) {
        (SizeMode::Width, _) => (bw, scale(h, w, bw)),
        (SizeMode::Height, _) => (scale(w, h, bh), bh),
        (_, Some((xw, xh))) => match o.fit {
            Fit::Stretch | Fit::Cover | Fit::Pad => (xw, xh),
            Fit::Contain => contain(w, h, xw, xh),
        },
        (_, None) => (w, h),
    };
    let (tw, th) = (tw.min(MAX_SIDE), th.min(MAX_SIDE));
    let pad = o.fit == Fit::Pad && target_box(o).is_some();
    if o.no_upscale && !pad && (tw > w || th > h) {
        (w, h)
    } else {
        (tw, th)
    }
}

/// 크기 단계 적용. `transparent`면 여백을 투명하게, 아니면 `bg`로 채운다.
pub(crate) fn resize_image(
    img: DynamicImage,
    o: &OptimizeOptions,
    transparent: bool,
    bg: [u8; 3],
) -> Result<DynamicImage, ToolError> {
    // 크기 단계가 꺼져 있으면 남아 있는 크기·맞춤 값과 상관없이 그대로
    if !o.resize {
        return Ok(img);
    }
    let (w, h) = (img.width(), img.height());
    let (tw, th) = target_size(w, h, o);
    ensure_pixels(tw, th)?;
    let boxed = target_box(o).is_some();
    if boxed && o.fit == Fit::Pad {
        let (mut iw, mut ih) = contain(w, h, tw, th);
        if o.no_upscale && (iw > w || ih > h) {
            (iw, ih) = (w, h);
        }
        let inner = if (iw, ih) == (w, h) {
            img
        } else {
            img.resize_exact(iw, ih, FilterType::Lanczos3)
        };
        let fill = if transparent {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([bg[0], bg[1], bg[2], 255])
        };
        let mut canvas = RgbaImage::from_pixel(tw, th, fill);
        let (x, y) = (
            i64::from((tw - iw.min(tw)) / 2),
            i64::from((th - ih.min(th)) / 2),
        );
        image::imageops::overlay(&mut canvas, &inner.to_rgba8(), x, y);
        return Ok(DynamicImage::ImageRgba8(canvas));
    }
    Ok(if (tw, th) == (w, h) {
        img
    } else if boxed && o.fit == Fit::Cover {
        img.resize_to_fill(tw, th, FilterType::Lanczos3)
    } else {
        img.resize_exact(tw, th, FilterType::Lanczos3)
    })
}
