//! JPG 무손실 회전·뒤집기 (mozjpeg-sys의 jpegtran 변환 코드 + src/lt_transform.c)

use std::os::raw::{c_int, c_ulong};
use std::path::Path;

use image::metadata::Orientation;
use localtools_image_common::{Fmt, Loaded};
// transupp.c가 쓰는 libjpeg 본체를 링크한다
use mozjpeg_sys as _;

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

/// C 래퍼 op 번호
pub(crate) const FLIP_H: c_int = 1;
pub(crate) const FLIP_V: c_int = 2;
pub(crate) const ROT_90: c_int = 3;
pub(crate) const ROT_180: c_int = 4;
pub(crate) const ROT_270: c_int = 5;

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
pub(crate) fn lossless_jpeg(
    path: &Path,
    src: &Loaded,
    ops: &[c_int],
    keep_markers: bool,
) -> Option<Vec<u8>> {
    if src.format != Fmt::Jpeg || ops.is_empty() {
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
    for &op in ops {
        bytes = transform_once(&bytes, op, keep_markers)?;
    }
    Some(bytes)
}
