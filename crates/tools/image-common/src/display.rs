//! 화면용 이미지 (썸네일·미리보기): 방향 반영, 줄이기, 흰 배경 합성, JPEG

use std::path::Path;

use waple_tools_core::ToolError;

use crate::{Fmt, Quality, encode, flatten, load};

pub struct DisplayImage {
    /// 화면용으로 줄인 JPEG
    pub jpeg: Vec<u8>,
    /// 원본 크기 (방향 반영 후)
    pub width: u32,
    pub height: u32,
}

/// 긴 변이 `max_side`보다 크면 줄여서 JPEG 85로 만든다. 투명 부분은 흰색.
pub fn display_image(path: &Path, max_side: u32) -> Result<DisplayImage, ToolError> {
    let src = load(path)?;
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
