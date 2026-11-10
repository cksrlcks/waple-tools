//! 용량 단계: 목표 용량 탐색, PNG 색상 수 줄이기 (옛 압축 도구에서 옮김)

use std::io::Cursor;

use image::{DynamicImage, RgbaImage};
use localtools_core::ToolError;
use localtools_image_common::optimize_png;

/// 품질 1~100 이진 탐색: `target` 바이트 이하인 가장 높은 품질의 결과.
/// 품질 1로도 크면 품질 1 결과(가장 작은 결과)를 돌려준다. 인코딩은 최대 8번.
pub(crate) fn search_quality(
    target: usize,
    encode: impl Fn(u8) -> Result<Vec<u8>, ToolError>,
) -> Result<Vec<u8>, ToolError> {
    let (mut lo, mut hi) = (1u8, 100u8);
    let mut best: Option<Vec<u8>> = None;
    let mut at_one: Option<Vec<u8>> = None;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let bytes = encode(mid)?;
        if bytes.len() <= target {
            best = Some(bytes);
            lo = mid + 1;
        } else {
            if mid == 1 {
                at_one = Some(bytes);
                break;
            }
            hi = mid - 1;
        }
    }
    match (best, at_one) {
        (Some(b), _) | (None, Some(b)) => Ok(b),
        (None, None) => encode(1),
    }
}

/// PNG 색상 수 줄이기. 완전 투명/불투명만 있으면 불투명 픽셀의 색만 줄이고,
/// 반투명 픽셀이 있으면 줄이지 않는다. oxipng가 256색 이하를 팔레트 PNG로 바꾼다.
fn reduce_colors(img: &DynamicImage, colors: u16) -> DynamicImage {
    let rgba = img.to_rgba8();
    if rgba.pixels().any(|p| p.0[3] != 0 && p.0[3] != 255) {
        return img.clone();
    }
    let has_transparent = rgba.pixels().any(|p| p.0[3] == 0);
    // 투명 칸 하나를 남겨 둔다
    let size = if has_transparent { colors - 1 } else { colors }.clamp(2, 256);
    let Ok(palette_size) = size.try_into() else {
        return img.clone();
    };
    let Ok(buf) = quantette::ImageBuf::try_from(img.to_rgb8()) else {
        return img.clone();
    };
    let quantized = quantette::Pipeline::new()
        .palette_size(palette_size)
        .input_image(buf.as_ref())
        .output_srgb8_image();
    let colors: Vec<[u8; 3]> = quantized
        .as_slice()
        .iter()
        .map(|c| [c.red, c.green, c.blue])
        .collect();
    let out = RgbaImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        if rgba.get_pixel(x, y).0[3] == 0 {
            image::Rgba([0, 0, 0, 0])
        } else {
            let [r, g, b] = colors[(y * rgba.width() + x) as usize];
            image::Rgba([r, g, b, 255])
        }
    });
    DynamicImage::ImageRgba8(out)
}

/// PNG 저장. `colors`가 2 이상이면 색상 수를 줄인 뒤 oxipng(`preset`)
pub(crate) fn encode_png(
    img: &DynamicImage,
    colors: u16,
    preset: u8,
) -> Result<Vec<u8>, ToolError> {
    let img = if colors >= 2 {
        reduce_colors(img, colors.min(256))
    } else {
        img.clone()
    };
    let mut raw = Vec::new();
    img.write_to(&mut Cursor::new(&mut raw), image::ImageFormat::Png)
        .map_err(|_| ToolError::Failed("이미지를 저장할 수 없습니다.".into()))?;
    optimize_png(&raw, preset)
}
