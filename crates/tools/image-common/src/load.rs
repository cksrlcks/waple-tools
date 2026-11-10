use std::path::Path;

use image::DynamicImage;
use image::metadata::Orientation;
use localtools_core::ToolError;

use crate::Fmt;
use crate::meta::{Meta, read_meta};

pub struct Loaded {
    /// EXIF 방향이 이미 반영된 픽셀
    pub image: DynamicImage,
    pub format: Fmt,
    pub meta: Meta,
}

fn unreadable() -> ToolError {
    ToolError::Failed("이미지를 읽을 수 없습니다.".into())
}

/// 이미지를 읽고 EXIF 방향을 픽셀에 반영한다. 포맷은 확장자가 아니라 내용으로 판별한다.
pub fn load(path: &Path) -> Result<Loaded, ToolError> {
    let bytes = std::fs::read(path)?;
    let format = image::guess_format(&bytes)
        .ok()
        .and_then(Fmt::from_image)
        .ok_or_else(unreadable)?;
    let mut image =
        image::load_from_memory_with_format(&bytes, format.to_image()).map_err(|_| unreadable())?;
    let meta = read_meta(&bytes);
    let orientation = meta
        .exif
        .as_deref()
        .and_then(Orientation::from_exif_chunk)
        .unwrap_or(Orientation::NoTransforms);
    image.apply_orientation(orientation);
    Ok(Loaded {
        image,
        format,
        meta,
    })
}
