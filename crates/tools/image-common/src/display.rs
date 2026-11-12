//! 화면용 이미지 (썸네일·미리보기): 방향 반영, 줄이기, 흰 배경 합성, JPEG · 미리보기용 가운데 조각

use std::path::Path;

use image::DynamicImage;
use waple_tools_core::ToolError;

use crate::load::load_with;
use crate::{Fmt, Quality, encode, flatten};

pub struct DisplayImage {
    /// 화면용으로 줄인 JPEG
    pub jpeg: Vec<u8>,
    /// 원본 크기 (방향 반영 후)
    pub width: u32,
    pub height: u32,
}

/// 긴 변이 `max_side`보다 크면 줄여서 JPEG 85로 만든다. 투명 부분은 흰색. 움직이는 GIF는 첫 프레임.
pub fn display_image(path: &Path, max_side: u32) -> Result<DisplayImage, ToolError> {
    let src = load_with(path, true)?;
    let (width, height) = (src.image.width(), src.image.height());
    let img = if width.max(height) > max_side {
        src.image.thumbnail(max_side, max_side)
    } else {
        src.image
    };
    let img = flatten(&img, [255, 255, 255]);
    let jpeg = encode(
        &img,
        Fmt::Jpeg,
        Quality {
            quality: 85,
            lossless: false,
        },
    )?;
    Ok(DisplayImage {
        jpeg,
        width,
        height,
    })
}

/// 느린 도구의 미리보기용 가운데 조각 (`x-preview-crop`): EXIF 방향 반영·알파 유지, 가운데 `min(n,w)×min(n,h)`.
/// 가로·세로 모두 `n` 이하면 `None` (원본 그대로 미리 본다)
pub fn center_crop(path: &Path, n: u32) -> Result<Option<DynamicImage>, ToolError> {
    let img = load_with(path, true)?.image;
    let (w, h) = (img.width(), img.height());
    if w <= n && h <= n {
        return Ok(None);
    }
    let (cw, ch) = (w.min(n), h.min(n));
    Ok(Some(img.crop_imm((w - cw) / 2, (h - ch) / 2, cw, ch)))
}
