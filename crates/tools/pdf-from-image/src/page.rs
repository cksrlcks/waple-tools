//! 이미지 한 장 → 페이지 하나 (이미지 XObject + 그리는 내용 스트림)

use image::DynamicImage;
use image::metadata::Orientation as ExifOrientation;
use waple_tools_core::ToolError;
use waple_tools_image_common::{Fmt, Quality, encode, flatten, load};
use waple_tools_pdf_common::lopdf::{Document, Object, ObjectId, Stream, dictionary};

use crate::{FromImageOptions, Orientation, PageSize};

const A4: (f32, f32) = (595.28, 841.89);
const LETTER: (f32, f32) = (612.0, 792.0);
/// "이미지 크기 그대로"의 기준 해상도
const IMAGE_DPI: f32 = 96.0;

/// JPEG SOF의 성분 수 (1 = 흑백, 3 = 컬러, 4 = CMYK)
fn jpeg_components(bytes: &[u8]) -> Option<u8> {
    let mut i = 2;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            return None;
        }
        let marker = bytes[i + 1];
        let len = usize::from(u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]));
        // SOF0~SOF15 (DHT C4, JPG C8, DAC CC 제외)
        if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
            return bytes.get(i + 9).copied();
        }
        i += 2 + len;
    }
    None
}

/// 이미지 XObject 스트림
fn image_stream(path: &std::path::Path, o: &FromImageOptions) -> Result<Stream, ToolError> {
    let bytes = std::fs::read(path)?;
    let loaded = load(path)?;
    let img = loaded.image;
    let (w, h) = (img.width() as i64, img.height() as i64);
    let rotated = loaded
        .meta
        .exif
        .as_deref()
        .and_then(ExifOrientation::from_exif_chunk)
        .is_some_and(|r| r != ExifOrientation::NoTransforms);
    let dict = |cs: &str, filter: &str| {
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => w,
            "Height" => h,
            "ColorSpace" => cs.to_string(),
            "BitsPerComponent" => 8,
            "Filter" => filter.to_string(),
        }
    };
    // JPG는 다시 압축하지 않으면 그대로 (회전 정보가 있거나 CMYK면 픽셀로 다시 만든다)
    if loaded.format == Fmt::Jpeg && !o.recompress && !rotated {
        let cs = match jpeg_components(&bytes) {
            Some(1) => Some("DeviceGray"),
            Some(3) => Some("DeviceRGB"),
            _ => None,
        };
        if let Some(cs) = cs {
            return Ok(Stream::new(dict(cs, "DCTDecode"), bytes).with_compression(false));
        }
    }
    if o.recompress {
        let q = Quality {
            quality: o.quality.clamp(30, 100),
            lossless: false,
        };
        let jpeg = encode(&img, Fmt::Jpeg, q)?;
        return Ok(Stream::new(dict("DeviceRGB", "DCTDecode"), jpeg).with_compression(false));
    }
    // 손실 없이: 투명 부분은 흰색, 흑백은 흑백 그대로
    let (cs, raw) = match &img {
        DynamicImage::ImageLuma8(g) => ("DeviceGray", g.as_raw().clone()),
        _ if img.color().has_alpha() => (
            "DeviceRGB",
            flatten(&img, [255, 255, 255]).to_rgb8().into_raw(),
        ),
        _ => ("DeviceRGB", img.to_rgb8().into_raw()),
    };
    let mut d = dict(cs, "FlateDecode");
    d.remove(b"Filter");
    let mut s = Stream::new(d, raw);
    s.compress()
        .map_err(|_| ToolError::Failed("이미지를 압축할 수 없습니다.".into()))?;
    // 압축은 끝났으니 저장할 때 다시 하지 않는다
    Ok(s.with_compression(false))
}

/// 페이지 (가로, 세로) pt
fn page_size(o: &FromImageOptions, w: f32, h: f32) -> (f32, f32) {
    let base = match o.page_size {
        PageSize::A4 => A4,
        PageSize::Letter => LETTER,
        PageSize::Image => return (w * 72.0 / IMAGE_DPI, h * 72.0 / IMAGE_DPI),
    };
    let landscape = match o.orientation {
        Orientation::Auto => w > h,
        Orientation::Portrait => false,
        Orientation::Landscape => true,
    };
    if landscape { (base.1, base.0) } else { base }
}

/// 페이지 안 이미지 자리 (x, y, 가로, 세로) pt: 여백 안에 비율 유지로 맞추고 가운데
fn place(page: (f32, f32), margin: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let aw = (page.0 - 2.0 * margin).max(1.0);
    let ah = (page.1 - 2.0 * margin).max(1.0);
    let s = (aw / w).min(ah / h);
    let (dw, dh) = (w * s, h * s);
    ((page.0 - dw) / 2.0, (page.1 - dh) / 2.0, dw, dh)
}

pub fn add(
    doc: &mut Document,
    parent: ObjectId,
    path: &std::path::Path,
    o: &FromImageOptions,
) -> Result<ObjectId, ToolError> {
    let img = image_stream(path, o)?;
    let n = |k: &[u8]| img.dict.get(k).and_then(Object::as_i64).unwrap_or(1) as f32;
    let (w, h) = (n(b"Width"), n(b"Height"));
    let size = page_size(o, w, h);
    let margin = if o.page_size == PageSize::Image {
        0.0
    } else {
        f32::from(o.margin.min(50)) * 72.0 / 25.4
    };
    let (x, y, dw, dh) = place(size, margin, w, h);
    let img_id = doc.add_object(img);
    let content = format!("q {dw:.3} 0 0 {dh:.3} {x:.3} {y:.3} cm /Im0 Do Q");
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    Ok(doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => parent,
        "MediaBox" => vec![0.into(), 0.into(), size.0.into(), size.1.into()],
        "Contents" => content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => img_id } },
    }))
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn place_fits_and_centers() {
        let (x, y, w, h) = place((200.0, 100.0), 10.0, 400.0, 400.0);
        assert_eq!((x, y, w, h), (60.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn auto_orientation_follows_image() {
        let o = FromImageOptions::default();
        assert_eq!(page_size(&o, 300.0, 200.0), (A4.1, A4.0));
        assert_eq!(page_size(&o, 200.0, 300.0), A4);
    }
}
