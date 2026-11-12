use std::path::Path;

use image::DynamicImage;
use image::metadata::Orientation;
use waple_tools_core::ToolError;

use crate::Fmt;
use crate::meta::{Meta, read_meta};

pub struct Loaded {
    /// EXIF 방향이 이미 반영된 픽셀
    pub image: DynamicImage,
    pub format: Fmt,
    pub meta: Meta,
}

fn is_animated_gif(bytes: &[u8]) -> bool {
    use image::AnimationDecoder;
    image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
        .map(|d| d.into_frames().take(2).count() > 1)
        .unwrap_or(false)
}

fn unreadable() -> ToolError {
    ToolError::Failed("이미지를 읽을 수 없습니다.".into())
}

/// 이미지를 읽고 EXIF 방향을 픽셀에 반영한다. 포맷은 확장자가 아니라 내용으로 판별한다.
pub fn load(path: &Path) -> Result<Loaded, ToolError> {
    load_with(path, false)
}

/// `allow_animated`이면 움직이는 GIF도 첫 프레임으로 읽는다 (썸네일·미리보기용)
pub fn load_with(path: &Path, allow_animated: bool) -> Result<Loaded, ToolError> {
    // 앞부분만 보고 이미지가 아니면 바로 실패한다 (큰 영상을 통째로 읽지 않게)
    let mut head = [0u8; 64];
    let n = std::io::Read::read(&mut std::fs::File::open(path)?, &mut head)?;
    if image::guess_format(&head[..n])
        .ok()
        .and_then(Fmt::from_image)
        .is_none()
    {
        return Err(unreadable());
    }
    let bytes = std::fs::read(path)?;
    let format = image::guess_format(&bytes)
        .ok()
        .and_then(Fmt::from_image)
        .ok_or_else(unreadable)?;
    if !allow_animated && format == Fmt::Gif && is_animated_gif(&bytes) {
        // 첫 프레임만 남은 정지 이미지가 "완료"로 나오지 않게 막는다 (움직이는 GIF는 GIF 최적화 도구에서)
        return Err(ToolError::Failed(
            "움직이는 GIF는 지원하지 않습니다. GIF 최적화 도구를 써 주세요.".into(),
        ));
    }
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
