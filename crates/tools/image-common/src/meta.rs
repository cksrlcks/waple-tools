//! EXIF·ICC: 읽기, 붙이기, 방향 초기화, GPS 제거

use std::io::Cursor;

use exif::{Field, In, Reader, Tag, Value};
use img_parts::{Bytes, DynImage, ImageEXIF, ImageICC};
use localtools_core::ToolError;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::Fmt;

#[derive(Clone, Default, Debug)]
pub struct Meta {
    /// TIFF 형식 원본 EXIF (`Exif\0\0` 접두사 없음)
    pub exif: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
}

/// 결과 파일의 메타데이터 처리
#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum MetaMode {
    /// 지우기
    #[default]
    Strip,
    /// 유지
    Keep,
}

const EXIF_PREFIX: &[u8] = b"Exif\0\0";

pub(crate) fn raw_tiff(b: &[u8]) -> Vec<u8> {
    b.strip_prefix(EXIF_PREFIX).unwrap_or(b).to_vec()
}

/// JPEG·PNG·WebP 바이트에서 EXIF·ICC를 읽는다. 그 밖의 포맷은 비어 있음.
pub(crate) fn read_meta(bytes: &[u8]) -> Meta {
    match DynImage::from_bytes(Bytes::copy_from_slice(bytes)) {
        Ok(Some(img)) => Meta {
            exif: img.exif().map(|e| raw_tiff(&e)),
            icc: img.icc_profile().map(|b| b.to_vec()),
        },
        _ => Meta::default(),
    }
}

/// EXIF에서 IFD0만 골라 다시 쓴다 (썸네일 IFD는 버린다). `edit`으로 필드를 바꾸거나 뺄 수 있다.
fn rewrite(exif: &[u8], edit: impl Fn(&Field) -> Option<Field>) -> Result<Vec<u8>, ToolError> {
    let bad = |_| ToolError::Failed("EXIF를 처리할 수 없습니다.".into());
    let parsed = Reader::new().read_raw(raw_tiff(exif)).map_err(bad)?;
    let fields: Vec<Field> = parsed
        .fields()
        .filter(|f| f.ifd_num == In::PRIMARY)
        .filter_map(&edit)
        .collect();
    let mut w = exif::experimental::Writer::new();
    for f in &fields {
        w.push_field(f);
    }
    let mut out = Cursor::new(Vec::new());
    w.write(&mut out, parsed.little_endian()).map_err(bad)?;
    Ok(out.into_inner())
}

/// 방향(0x0112)을 1로 바꾼 사본. 읽을 수 없는 EXIF면 그대로 돌려준다.
pub fn exif_reset_orientation(exif: &[u8]) -> Vec<u8> {
    rewrite(exif, |f| {
        if f.tag == Tag::Orientation {
            Some(Field {
                tag: f.tag,
                ifd_num: f.ifd_num,
                value: Value::Short(vec![1]),
            })
        } else {
            Some(f.clone())
        }
    })
    .unwrap_or_else(|_| raw_tiff(exif))
}

/// GPS 필드를 뺀 EXIF
pub fn exif_without_gps(exif: &[u8]) -> Result<Vec<u8>, ToolError> {
    rewrite(exif, |f| {
        let gps = f.tag.context() == exif::Context::Gps;
        (!gps).then(|| f.clone())
    })
}

/// 인코딩한 JPEG·PNG·WebP에 EXIF(방향 1로 초기화)와 ICC를 붙인다. 그 밖의 포맷은 그대로.
pub fn with_meta(bytes: Vec<u8>, fmt: Fmt, meta: &Meta) -> Result<Vec<u8>, ToolError> {
    if !matches!(fmt, Fmt::Jpeg | Fmt::Png | Fmt::Webp) {
        return Ok(bytes);
    }
    fn fail<E>(_: E) -> ToolError {
        ToolError::Failed("메타데이터를 붙일 수 없습니다.".into())
    }
    let Some(mut img) = DynImage::from_bytes(Bytes::from(bytes.clone())).map_err(fail)? else {
        return Ok(bytes);
    };
    if let Some(e) = &meta.exif {
        img.set_exif(Some(Bytes::from(exif_reset_orientation(e))));
    }
    if let Some(icc) = &meta.icc {
        img.set_icc_profile(Some(Bytes::from(icc.clone())));
    }
    let mut out = Vec::new();
    img.encoder().write_to(&mut out).map_err(fail)?;
    Ok(out)
}
