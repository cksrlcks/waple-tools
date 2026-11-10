use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use image::{DynamicImage, Rgb, RgbImage};
use localtools_core::ToolError;

use crate::Fmt;

#[derive(Clone, Copy, Debug)]
pub struct Quality {
    /// 1~100 (JPEG·WebP 손실)
    pub quality: u8,
    /// WebP 무손실
    pub lossless: bool,
}

fn failed() -> ToolError {
    ToolError::Failed("이미지를 저장할 수 없습니다.".into())
}

/// 포맷별 인코딩. JPEG는 mozjpeg, PNG는 image + oxipng, WebP는 libwebp.
/// JPEG에 알파가 있으면 먼저 `flatten`할 것 (여기서는 흰색으로 합성한다).
pub fn encode(img: &DynamicImage, fmt: Fmt, q: Quality) -> Result<Vec<u8>, ToolError> {
    match fmt {
        Fmt::Jpeg => encode_jpeg(img, q.quality),
        Fmt::Png => {
            let mut raw = Vec::new();
            img.write_to(&mut Cursor::new(&mut raw), image::ImageFormat::Png)
                .map_err(|_| failed())?;
            optimize_png(&raw, 2)
        }
        Fmt::Webp => {
            let rgba;
            let src = if img.color().has_alpha() {
                rgba = DynamicImage::ImageRgba8(img.to_rgba8());
                &rgba
            } else {
                rgba = DynamicImage::ImageRgb8(img.to_rgb8());
                &rgba
            };
            let enc = webp::Encoder::from_image(src).map_err(|_| failed())?;
            let mem = if q.lossless {
                enc.encode_lossless()
            } else {
                enc.encode(f32::from(q.quality.clamp(1, 100)))
            };
            Ok(mem.to_vec())
        }
        Fmt::Gif | Fmt::Bmp | Fmt::Tiff => {
            let mut out = Vec::new();
            let src = if fmt == Fmt::Gif {
                DynamicImage::ImageRgba8(img.to_rgba8())
            } else {
                img.clone()
            };
            src.write_to(&mut Cursor::new(&mut out), fmt.to_image())
                .map_err(|_| failed())?;
            Ok(out)
        }
    }
}

/// oxipng 무손실 최적화. 메타데이터 청크는 지운다(필요하면 나중에 `with_meta`로 붙인다).
pub fn optimize_png(raw: &[u8], preset: u8) -> Result<Vec<u8>, ToolError> {
    let mut opts = oxipng::Options::from_preset(preset);
    opts.strip = oxipng::StripChunks::Safe;
    oxipng::optimize_from_memory(raw, &opts).map_err(|_| failed())
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ToolError> {
    let rgb = if img.color().has_alpha() {
        flatten(img, [255, 255, 255]).to_rgb8()
    } else {
        img.to_rgb8()
    };
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    // mozjpeg는 libjpeg 오류를 패닉으로 알린다
    catch_unwind(AssertUnwindSafe(|| -> std::io::Result<Vec<u8>> {
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(f32::from(quality.clamp(1, 100)));
        let mut started = c.start_compress(Vec::new())?;
        started.write_scanlines(rgb.as_raw())?;
        started.finish()
    }))
    .map_err(|_| failed())?
    .map_err(|_| failed())
}

/// 알파를 배경색 위에 합성해 RGB로 만든다 (JPEG 저장용)
pub fn flatten(img: &DynamicImage, bg: [u8; 3]) -> DynamicImage {
    let rgba = img.to_rgba8();
    let out = RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = u16::from(p[3]);
        let mix = |c: u8, b: u8| ((u16::from(c) * a + u16::from(b) * (255 - a)) / 255) as u8;
        Rgb([mix(p[0], bg[0]), mix(p[1], bg[1]), mix(p[2], bg[2])])
    });
    DynamicImage::ImageRgb8(out)
}
