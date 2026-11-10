//! 메타데이터 제거 (기본 내장): 다시 인코딩하지 않고 메타데이터 블록만 지운다

use std::path::{Path, PathBuf};

use img_parts::{Bytes, DynImage, ImageEXIF, ImageICC};
use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, exif_without_gps, parse_options, run_each, schema_of, write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Scope {
    /// 전체 (EXIF·XMP·주석)
    #[default]
    All,
    /// GPS 위치만
    GpsOnly,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct StripOptions {
    /// 제거 범위
    scope: Scope,
    /// 색상 프로파일 유지
    keep_icc: bool,
}

impl Default for StripOptions {
    fn default() -> Self {
        Self {
            scope: Scope::All,
            keep_icc: true,
        }
    }
}

const JPEG_APP1: u8 = 0xE1; // EXIF·XMP
const JPEG_APP13: u8 = 0xED; // IPTC·Photoshop
const JPEG_COM: u8 = 0xFE; // 주석

fn unsupported() -> ToolError {
    ToolError::Failed("이 형식은 메타데이터 제거를 지원하지 않습니다.".into())
}

fn strip(input: &Path, out_dir: &Path, o: &StripOptions) -> Result<PathBuf, ToolError> {
    let bytes = std::fs::read(input)?;
    let mut img = DynImage::from_bytes(Bytes::from(bytes))
        .map_err(|_| unsupported())?
        .ok_or_else(unsupported)?;
    match o.scope {
        Scope::GpsOnly => {
            if let Some(exif) = img.exif() {
                img.set_exif(Some(Bytes::from(exif_without_gps(&exif)?)));
            }
        }
        Scope::All => {
            img.set_exif(None);
            match &mut img {
                DynImage::Jpeg(j) => {
                    for m in [JPEG_APP1, JPEG_APP13, JPEG_COM] {
                        j.remove_segments_by_marker(m);
                    }
                }
                DynImage::Png(p) => {
                    for t in [*b"tEXt", *b"iTXt", *b"zTXt", *b"tIME"] {
                        p.remove_chunks_by_type(t);
                    }
                }
                DynImage::WebP(w) => w.remove_chunks_by_id(*b"XMP "),
            }
        }
    }
    if !o.keep_icc {
        img.set_icc_profile(None);
    }
    let fmt = match &img {
        DynImage::Jpeg(_) => Fmt::Jpeg,
        DynImage::Png(_) => Fmt::Png,
        DynImage::WebP(_) => Fmt::Webp,
    };
    let mut out = Vec::new();
    img.encoder()
        .write_to(&mut out)
        .map_err(|_| ToolError::Failed("이미지를 저장할 수 없습니다.".into()))?;
    write_output(out_dir, input, fmt, &out)
}

struct StripMeta;

pub fn tool() -> Box<dyn Tool> {
    Box::new(StripMeta)
}

impl Tool for StripMeta {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.strip_meta".into(),
            name: "메타데이터 제거".into(),
            category: "image".into(),
            description: "EXIF·GPS 정보를 지웁니다.".into(),
            extensions: ["jpg", "jpeg", "png", "webp"].map(String::from).to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<StripOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: StripOptions = parse_options(opts)?;
        run_each(ctx, input, |p| strip(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exif::{In, Reader, Tag};
    use localtools_core::{JobContext, ToolError, ToolOutput};
    use localtools_image_common::load;
    use localtools_image_common::testkit::{jpeg_with_exif, photo};
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

    #[test]
    fn all_removes_exif_and_keeps_pixels() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        jpeg_with_exif(&src, 32, 16, 1, true);
        let out = run(&src, &d.path().join("o"), json!({})).unwrap();
        assert_eq!(out.file_name().unwrap(), "a.jpg");
        assert!(load(&out).unwrap().meta.exif.is_none());
        let a = image::open(&src).unwrap().to_rgb8();
        let b = image::open(&out).unwrap().to_rgb8();
        assert_eq!(a, b, "다시 인코딩하지 않으므로 픽셀이 같아야 함");
        assert!(std::fs::metadata(&out).unwrap().len() < std::fs::metadata(&src).unwrap().len());
    }

    #[test]
    fn gps_only_keeps_other_exif() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        jpeg_with_exif(&src, 16, 16, 3, true);
        let out = run(&src, &d.path().join("o"), json!({ "scope": "gpsOnly" })).unwrap();
        let exif = Reader::new()
            .read_raw(load(&out).unwrap().meta.exif.unwrap())
            .unwrap();
        assert!(exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY).is_none());
        assert!(exif.get_field(Tag::Orientation, In::PRIMARY).is_some());
    }

    #[test]
    fn keep_icc_false_removes_icc() {
        use img_parts::{Bytes, DynImage, ImageICC};
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(8, 8).save(&src).unwrap();
        let mut img = DynImage::from_bytes(Bytes::from(std::fs::read(&src).unwrap()))
            .unwrap()
            .unwrap();
        img.set_icc_profile(Some(Bytes::from_static(b"fake-icc-profile")));
        let mut with_icc = Vec::new();
        img.encoder().write_to(&mut with_icc).unwrap();
        std::fs::write(&src, with_icc).unwrap();

        let kept = run(&src, &d.path().join("k"), json!({})).unwrap();
        assert!(
            load(&kept).unwrap().meta.icc.is_some(),
            "기본은 색상 프로파일 유지"
        );
        let gone = run(&src, &d.path().join("g"), json!({ "keepIcc": false })).unwrap();
        assert!(load(&gone).unwrap().meta.icc.is_none());
    }

    #[test]
    fn bmp_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.bmp");
        photo(4, 4).save(&src).unwrap();
        let r = run(&src, &d.path().join("o"), json!({}));
        assert!(
            matches!(r, Err(ToolError::Failed(m)) if m == "이 형식은 메타데이터 제거를 지원하지 않습니다.")
        );
        assert_eq!(tool().meta().name, "메타데이터 제거");
    }
}
