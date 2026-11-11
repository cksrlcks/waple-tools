//! 강하게: PDF 안 사진을 최대 DPI로 줄이고 JPEG로 다시 압축한다. 더 작아질 때만 바꾼다.

use std::collections::HashSet;

use image::{DynamicImage, GrayImage, RgbImage, imageops::FilterType};
use waple_tools_core::{JobContext, ToolError};
use waple_tools_image_common::{Fmt, MAX_PIXELS, Quality, encode};
use waple_tools_pdf_common::lopdf::{Dictionary, Document, Object, ObjectId, Stream};

/// 압축을 풀었을 때 이보다 크면 손대지 않는다 (압축 폭탄 방지)
const MAX_RAW: usize = 512 * 1024 * 1024;

fn name<'a>(d: &'a Dictionary, key: &[u8]) -> Option<&'a [u8]> {
    d.get(key).and_then(Object::as_name).ok()
}

fn int(d: &Dictionary, key: &[u8]) -> Option<i64> {
    d.get(key).and_then(Object::as_i64).ok()
}

/// 가장 큰 페이지의 (긴 변, 짧은 변) 인치. 없으면 Letter
fn page_inches(doc: &Document) -> (f32, f32) {
    let mut best = (11.0_f32, 8.5_f32);
    let mut area = 0.0;
    for id in doc.get_pages().values() {
        let Ok(page) = doc.get_dictionary(*id) else {
            continue;
        };
        let Some(b) = page
            .get(b"MediaBox")
            .and_then(Object::as_array)
            .ok()
            .filter(|b| b.len() == 4)
        else {
            continue;
        };
        let v: Vec<f32> = b.iter().filter_map(|o| o.as_float().ok()).collect();
        if v.len() != 4 {
            continue;
        }
        let (w, h) = ((v[2] - v[0]).abs() / 72.0, (v[3] - v[1]).abs() / 72.0);
        if w * h > area {
            area = w * h;
            best = (w.max(h), w.min(h));
        }
    }
    best
}

/// 다룰 수 있는 사진이면 픽셀로 푼다: 8비트 RGB·Gray, JPEG 또는 Flate 계열
fn decode(s: &Stream) -> Option<DynamicImage> {
    let d = &s.dict;
    if name(d, b"Subtype") != Some(b"Image")
        || d.get(b"ImageMask").and_then(Object::as_bool).unwrap_or(false)
        || d.has(b"Decode")
        // 색 키 마스크(배열)는 손실 압축하면 맞지 않는다
        || d.get(b"Mask").is_ok_and(|m| m.as_array().is_ok())
        || int(d, b"BitsPerComponent") != Some(8)
    {
        return None;
    }
    let comps = match name(d, b"ColorSpace")? {
        b"DeviceRGB" => 3,
        b"DeviceGray" => 1,
        _ => return None,
    };
    let (w, h) = (int(d, b"Width")?, int(d, b"Height")?);
    if w <= 0 || h <= 0 || (w * h) as u64 > MAX_PIXELS {
        return None;
    }
    let (w, h) = (w as u32, h as u32);
    let filters = s.filters().ok()?;
    if filters.as_slice() == [b"DCTDecode".as_slice()] {
        let img = image::load_from_memory_with_format(&s.content, image::ImageFormat::Jpeg).ok()?;
        return (img.width() == w && img.height() == h).then_some(img);
    }
    if filters
        .iter()
        .any(|f| *f == b"DCTDecode" || *f == b"JPXDecode")
    {
        return None;
    }
    let raw = s.decompressed_content_with_limit(MAX_RAW).ok()?;
    let need = (w as usize) * (h as usize) * comps;
    if raw.len() < need {
        return None;
    }
    let raw = raw[..need].to_vec();
    if comps == 3 {
        RgbImage::from_raw(w, h, raw).map(DynamicImage::ImageRgb8)
    } else {
        GrayImage::from_raw(w, h, raw).map(DynamicImage::ImageLuma8)
    }
}

/// 다른 이미지의 마스크로 쓰이는 이미지 (손대지 않는다)
fn mask_ids(doc: &Document) -> HashSet<ObjectId> {
    doc.objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .flat_map(|s| [b"SMask".as_slice(), b"Mask"].map(|k| s.dict.get(k)))
        .filter_map(|o| o.ok()?.as_reference().ok())
        .collect()
}

/// 사진마다: DPI가 넘으면 줄이고 JPEG로. 원래보다 작을 때만 바꾼다.
// ponytail: 실제 배치 크기를 계산하지 않고 "사진이 가장 큰 페이지 전체를 덮는다"고 보고 DPI를 어림한다. 작게 배치된 사진은 덜 줄어든다
pub fn shrink(
    doc: &mut Document,
    max_dpi: u32,
    quality: u8,
    ctx: &JobContext,
) -> Result<(), ToolError> {
    let (long_in, short_in) = page_inches(doc);
    let masks = mask_ids(doc);
    let ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter(|(id, o)| {
            !masks.contains(id)
                && o.as_stream()
                    .is_ok_and(|s| name(&s.dict, b"Subtype") == Some(b"Image"))
        })
        .map(|(id, _)| *id)
        .collect();
    for (i, id) in ids.iter().enumerate() {
        ctx.check_cancel()?;
        ctx.progress(i as f32 / ids.len() as f32);
        let Some(Object::Stream(s)) = doc.objects.get(id) else {
            continue;
        };
        let Some(img) = decode(s) else { continue };
        let (w, h) = (img.width() as f32, img.height() as f32);
        let dpi = max_dpi as f32;
        let scale = (long_in * dpi / w.max(h))
            .min(short_in * dpi / w.min(h))
            .min(1.0);
        let img = if scale < 1.0 {
            let nw = ((w * scale).round() as u32).max(1);
            let nh = ((h * scale).round() as u32).max(1);
            img.resize_exact(nw, nh, FilterType::Lanczos3)
        } else {
            img
        };
        let jpeg = encode(
            &img,
            Fmt::Jpeg,
            Quality {
                quality,
                lossless: false,
            },
        )?;
        if jpeg.len() >= s.content.len() {
            continue;
        }
        let mut dict = s.dict.clone();
        dict.remove(b"DecodeParms");
        dict.set("Filter", "DCTDecode");
        dict.set("Width", img.width() as i64);
        dict.set("Height", img.height() as i64);
        // JPEG 인코더는 늘 RGB로 쓴다
        dict.set("ColorSpace", "DeviceRGB");
        dict.set("BitsPerComponent", 8);
        doc.objects.insert(
            *id,
            Object::Stream(Stream::new(dict, jpeg).with_compression(false)),
        );
    }
    Ok(())
}

/// 압축을 풀 수 있는 스트림은 풀어 둔다 (저장할 때 최고 수준으로 다시 압축). 너무 큰 것·못 푸는 것은 그대로.
pub fn decompress_all(doc: &mut Document) {
    for obj in doc.objects.values_mut() {
        if let Object::Stream(s) = obj
            && s.allows_compression
        {
            let _ = s.decompress_with_limit(MAX_RAW);
        }
    }
}
