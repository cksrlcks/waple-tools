//! 회전 / 뒤집기 (기본 내장). JPG는 가능하면 무손실(jpegtran 변환), 아니면 다시 저장.

use std::os::raw::{c_int, c_ulong};
use std::path::{Path, PathBuf};

use image::DynamicImage;
use image::metadata::Orientation;
use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, Loaded, MetaMode, Quality, encode, load, parse_options, run_each, schema_of,
    with_meta, write_output,
};
// transupp.c가 쓰는 libjpeg 본체를 링크한다
use mozjpeg_sys as _;
use schemars::JsonSchema;
use serde::Deserialize;

unsafe extern "C" {
    fn lt_jpeg_transform(
        input: *const u8,
        len: c_ulong,
        op: c_int,
        copy_all_markers: c_int,
        out: *mut *mut u8,
        out_len: *mut c_ulong,
    ) -> c_int;
    fn lt_free(p: *mut u8);
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Angle {
    /// 회전 안 함
    None,
    /// 90° (시계 방향)
    #[default]
    R90,
    /// 180°
    R180,
    /// 270° (시계 반대 90°)
    R270,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct RotateOptions {
    /// 회전
    angle: Angle,
    /// 좌우 뒤집기
    flip_h: bool,
    /// 상하 뒤집기
    flip_v: bool,
    /// 품질 (다시 저장할 때, JPG·WebP)
    #[schemars(range(min = 1, max = 100))]
    quality: u8,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for RotateOptions {
    fn default() -> Self {
        Self {
            angle: Angle::R90,
            flip_h: false,
            flip_v: false,
            quality: 95,
            metadata: MetaMode::Strip,
        }
    }
}

/// C 래퍼 op 번호: 1 좌우, 2 상하, 3 90°, 4 180°, 5 270° (회전 후 뒤집기 순서)
fn ops(o: &RotateOptions) -> Vec<c_int> {
    let mut v = Vec::new();
    match o.angle {
        Angle::None => {}
        Angle::R90 => v.push(3),
        Angle::R180 => v.push(4),
        Angle::R270 => v.push(5),
    }
    if o.flip_h {
        v.push(1);
    }
    if o.flip_v {
        v.push(2);
    }
    v
}

/// 한 번의 무손실 변환. 손실 없이 못 하면(MCU 경계·오류) None.
fn transform_once(jpeg: &[u8], op: c_int, keep_markers: bool) -> Option<Vec<u8>> {
    let mut out: *mut u8 = std::ptr::null_mut();
    let mut out_len: c_ulong = 0;
    let len = c_ulong::try_from(jpeg.len()).ok()?;
    // SAFETY: 입력은 살아 있는 슬라이스, 출력 버퍼는 성공했을 때만 C가 malloc해 넘겨 주고 바로 복사 후 해제한다
    let rc = unsafe {
        lt_jpeg_transform(
            jpeg.as_ptr(),
            len,
            op,
            c_int::from(keep_markers),
            &mut out,
            &mut out_len,
        )
    };
    if rc != 0 || out.is_null() {
        return None;
    }
    // SAFETY: rc == 0이면 out은 out_len 바이트짜리 유효한 버퍼
    let bytes = unsafe { std::slice::from_raw_parts(out, out_len as usize).to_vec() };
    unsafe { lt_free(out) };
    Some(bytes)
}

/// EXIF 방향이 없거나 1인 JPEG만 무손실로 돌린다 (방향 값과 섞이면 결과가 헷갈림)
fn lossless_jpeg(path: &Path, src: &Loaded, o: &RotateOptions) -> Option<Vec<u8>> {
    if src.format != Fmt::Jpeg {
        return None;
    }
    let oriented = src
        .meta
        .exif
        .as_deref()
        .and_then(Orientation::from_exif_chunk)
        .is_some_and(|x| x != Orientation::NoTransforms);
    if oriented {
        return None;
    }
    let mut bytes = std::fs::read(path).ok()?;
    for op in ops(o) {
        bytes = transform_once(&bytes, op, o.metadata == MetaMode::Keep)?;
    }
    Some(bytes)
}

fn reencode(src: Loaded, o: &RotateOptions) -> Result<Vec<u8>, ToolError> {
    let mut img: DynamicImage = match o.angle {
        Angle::None => src.image,
        Angle::R90 => src.image.rotate90(),
        Angle::R180 => src.image.rotate180(),
        Angle::R270 => src.image.rotate270(),
    };
    if o.flip_h {
        img = img.fliph();
    }
    if o.flip_v {
        img = img.flipv();
    }
    let q = Quality {
        quality: o.quality.clamp(1, 100),
        lossless: false,
    };
    let mut bytes = encode(&img, src.format, q)?;
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, src.format, &src.meta)?;
    }
    Ok(bytes)
}

fn rotate(input: &Path, out_dir: &Path, o: &RotateOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let fmt = src.format;
    let bytes = match lossless_jpeg(input, &src, o) {
        Some(b) => b,
        None => reencode(src, o)?,
    };
    write_output(out_dir, input, fmt, &bytes)
}

struct Rotate;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Rotate)
}

impl Tool for Rotate {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.rotate".into(),
            name: "회전 / 뒤집기".into(),
            category: "image".into(),
            description: "90°·180°·270° 회전하거나 뒤집습니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<RotateOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: RotateOptions = parse_options(opts)?;
        if ops(&o).is_empty() {
            return Err(ToolError::InvalidOptions(
                "회전이나 뒤집기를 하나 이상 고르세요.".into(),
            ));
        }
        run_each(ctx, input, |p| rotate(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, ToolError, ToolOutput};
    use localtools_image_common::testkit::{jpeg_with_exif, photo};
    use localtools_image_common::{Fmt, Quality, encode};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn run(input: &Path, out: &Path, v: serde_json::Value) -> Result<PathBuf, ToolError> {
        std::fs::create_dir_all(out).unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        match tool().run(&ctx, &[input.to_path_buf()], out, &v)? {
            ToolOutput::Files(f) => Ok(out.join(&f[0])),
            other => panic!("{other:?}"),
        }
    }

    fn dims(p: &Path) -> (u32, u32) {
        let i = image::open(p).unwrap();
        (i.width(), i.height())
    }

    #[test]
    fn r90_swaps_dimensions() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(40, 20).save(&src).unwrap();
        let out = run(&src, &d.path().join("o"), json!({ "angle": "r90" })).unwrap();
        assert_eq!(dims(&out), (20, 40));
    }

    #[test]
    fn flip_h_mirrors_pixel() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        let mut img = image::RgbImage::new(4, 2);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.save(&src).unwrap();
        let out = run(
            &src,
            &d.path().join("o"),
            json!({ "angle": "none", "flipH": true }),
        )
        .unwrap();
        let back = image::open(out).unwrap().to_rgb8();
        assert_eq!(back.get_pixel(3, 0).0, [255, 0, 0]);
        assert_eq!(back.get_pixel(0, 0).0, [0, 0, 0]);
    }

    #[test]
    fn noop_is_invalid() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(4, 4).save(&src).unwrap();
        let r = run(&src, &d.path().join("o"), json!({ "angle": "none" }));
        assert!(
            matches!(r, Err(ToolError::InvalidOptions(m)) if m == "회전이나 뒤집기를 하나 이상 고르세요.")
        );
    }

    #[test]
    fn jpeg_r90_is_lossless_when_mcu_aligned() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        let jpeg = encode(
            &photo(64, 32),
            Fmt::Jpeg,
            Quality {
                quality: 85,
                lossless: false,
            },
        )
        .unwrap();
        std::fs::write(&src, &jpeg).unwrap();
        let once = run(&src, &d.path().join("o1"), json!({ "angle": "r90" })).unwrap();
        assert_eq!(dims(&once), (32, 64));
        let back = run(&once, &d.path().join("o2"), json!({ "angle": "r270" })).unwrap();
        let a = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let b = image::open(&back).unwrap().to_rgb8();
        assert_eq!(a, b, "90°→270° 무손실 왕복이면 픽셀이 같아야 함");
    }

    #[test]
    fn jpeg_unaligned_falls_back() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        let jpeg = encode(
            &photo(65, 33),
            Fmt::Jpeg,
            Quality {
                quality: 85,
                lossless: false,
            },
        )
        .unwrap();
        std::fs::write(&src, jpeg).unwrap();
        let out = run(&src, &d.path().join("o"), json!({ "angle": "r90" })).unwrap();
        assert_eq!(dims(&out), (33, 65));
    }

    #[test]
    fn jpeg_with_orientation_falls_back_and_keeps_visual() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        // 저장 40×16, 방향 6 → 보이는 크기 16×40 → 90° 회전 → 40×16
        jpeg_with_exif(&src, 40, 16, 6, false);
        let out = run(&src, &d.path().join("o"), json!({ "angle": "r90" })).unwrap();
        assert_eq!(dims(&out), (40, 16));
        assert_eq!(tool().meta().name, "회전 / 뒤집기");
    }
}
