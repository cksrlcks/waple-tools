//! WebP 변환 (기본 내장)

use std::path::{Path, PathBuf};

use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, encode, load, parse_options, run_each, schema_of,
    with_meta, write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct WebpOptions {
    /// 품질
    #[schemars(range(min = 0, max = 100))]
    quality: u8,
    /// 무손실
    lossless: bool,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for WebpOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            lossless: false,
            metadata: MetaMode::Strip,
        }
    }
}

struct Webp;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Webp)
}

fn convert(input: &Path, out_dir: &Path, o: &WebpOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let q = Quality {
        quality: o.quality.max(1),
        lossless: o.lossless,
    };
    let mut bytes = encode(&src.image, Fmt::Webp, q)?;
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, Fmt::Webp, &src.meta)?;
    }
    write_output(out_dir, input, Fmt::Webp, &bytes)
}

impl Tool for Webp {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.webp".into(),
            name: "WebP 변환".into(),
            category: "image".into(),
            description: "이미지를 WebP로 변환합니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<WebpOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: WebpOptions = parse_options(opts)?;
        run_each(ctx, input, |p| convert(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exif::{In, Reader, Tag};
    use localtools_core::{JobContext, ToolOutput};
    use localtools_image_common::testkit::{jpeg_with_exif, photo};
    use localtools_image_common::{Fmt, load};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn run(input: &Path, out: &Path, opts: serde_json::Value) -> PathBuf {
        let ctx = JobContext::new(|_| {}, Default::default());
        match tool()
            .run(&ctx, &[input.to_path_buf()], out, &opts)
            .unwrap()
        {
            ToolOutput::Files(f) => f[0].clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn converts_png_to_webp() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(32, 24).save(&src).unwrap();
        let name = run(&src, d.path(), json!({}));
        assert_eq!(name, Path::new("a.webp"));
        let bytes = std::fs::read(d.path().join(&name)).unwrap();
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::WebP
        );
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!((img.width(), img.height()), (32, 24));
    }

    #[test]
    fn lossless_is_pixel_exact() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        let original = photo(16, 16);
        original.save(&src).unwrap();
        let name = run(&src, d.path(), json!({ "lossless": true }));
        let back = image::open(d.path().join(name)).unwrap().to_rgb8();
        assert_eq!(back, original.to_rgb8());
    }

    #[test]
    fn strip_by_default() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        jpeg_with_exif(&src, 16, 8, 1, true);
        let name = run(&src, d.path(), json!({}));
        assert!(load(&d.path().join(name)).unwrap().meta.exif.is_none());
    }

    #[test]
    fn keep_metadata_resets_orientation() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        jpeg_with_exif(&src, 16, 8, 6, false);
        let name = run(&src, d.path(), json!({ "metadata": "keep" }));
        let out = d.path().join(name);
        let loaded = load(&out).unwrap();
        assert_eq!(loaded.format, Fmt::Webp);
        assert_eq!((loaded.image.width(), loaded.image.height()), (8, 16));
        let exif = Reader::new().read_raw(loaded.meta.exif.unwrap()).unwrap();
        let o = exif.get_field(Tag::Orientation, In::PRIMARY).unwrap();
        assert_eq!(o.value.get_uint(0), Some(1));
    }

    #[test]
    fn schema_has_slider_for_quality() {
        let s = tool().options_schema();
        let q = &s["properties"]["quality"];
        assert_eq!(q["minimum"], 0);
        assert_eq!(q["maximum"], 100);
        assert_eq!(q["default"], 80);
    }

    #[test]
    fn meta_matches_catalog() {
        let m = tool().meta();
        assert_eq!(m.id, "image.webp");
        assert_eq!(m.name, "WebP 변환");
    }
}
