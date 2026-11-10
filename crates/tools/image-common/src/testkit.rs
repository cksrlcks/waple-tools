//! 테스트용 이미지 생성기 (feature "testkit")

use std::io::Cursor;
use std::path::Path;

use exif::{Field, In, Rational, Tag, Value};
use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};
use img_parts::{Bytes, ImageEXIF, jpeg::Jpeg};

/// 그라디언트 + 고정 시드 노이즈. 압축률이 품질에 반응하도록 사진처럼 만든다.
pub fn photo(w: u32, h: u32) -> DynamicImage {
    let mut seed: u32 = 0x1234_5678;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 48) as i32 - 24
    };
    let img = RgbImage::from_fn(w, h, |x, y| {
        let base = [
            (x * 255 / w.max(1)) as i32,
            (y * 255 / h.max(1)) as i32,
            ((x + y) * 127 / (w + h).max(1)) as i32,
        ];
        Rgb(base.map(|c| (c + noise()).clamp(0, 255) as u8))
    });
    DynamicImage::ImageRgb8(img)
}

/// 왼쪽 절반이 완전 투명, 오른쪽 절반이 불투명 파랑인 PNG
pub fn alpha_png(path: &Path, w: u32, h: u32) {
    let img = RgbaImage::from_fn(w, h, |x, _| {
        if x < w / 2 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([0, 0, 255, 255])
        }
    });
    img.save(path).unwrap();
}

/// 방향과 (선택) GPS가 든 EXIF를 붙인 JPEG. 픽셀은 저장된 그대로 w×h.
pub fn jpeg_with_exif(path: &Path, w: u32, h: u32, orientation: u16, gps: bool) {
    let mut jpeg = Vec::new();
    photo(w, h)
        .write_to(&mut Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .unwrap();

    let mut fields = vec![
        Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![orientation]),
        },
        Field {
            tag: Tag::Make,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![b"LocalTools".to_vec()]),
        },
    ];
    if gps {
        fields.push(Field {
            tag: Tag::GPSLatitudeRef,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![b"N".to_vec()]),
        });
        fields.push(Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 37, denom: 1 },
                Rational { num: 33, denom: 1 },
                Rational { num: 0, denom: 1 },
            ]),
        });
    }
    let mut w_ = exif::experimental::Writer::new();
    for f in &fields {
        w_.push_field(f);
    }
    let mut tiff = Cursor::new(Vec::new());
    w_.write(&mut tiff, false).unwrap();

    let mut j = Jpeg::from_bytes(Bytes::from(jpeg)).unwrap();
    j.set_exif(Some(Bytes::from(tiff.into_inner())));
    let mut out = Vec::new();
    j.encoder().write_to(&mut out).unwrap();
    std::fs::write(path, out).unwrap();
}

/// 프레임이 `frames`개인 움직이는 GIF
pub fn animated_gif(path: &Path, w: u32, h: u32, frames: u32) {
    use image::Frame;
    use image::codecs::gif::{GifEncoder, Repeat};
    let file = std::fs::File::create(path).unwrap();
    let mut enc = GifEncoder::new(file);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..frames {
        let c = (i * 80 % 255) as u8;
        enc.encode_frame(Frame::new(RgbaImage::from_pixel(
            w,
            h,
            Rgba([c, 0, 255 - c, 255]),
        )))
        .unwrap();
    }
}
