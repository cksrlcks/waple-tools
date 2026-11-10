//! 포맷 변환 (기본 내장)

use std::path::{Path, PathBuf};

use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, encode, flatten, load, parse_color, parse_options,
    run_each, schema_of, with_meta, write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum OutFmt {
    /// JPG
    Jpg,
    /// PNG
    #[default]
    Png,
    /// WebP
    Webp,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct ConvertOptions {
    /// 출력 포맷
    format: OutFmt,
    /// JPG 품질
    #[schemars(range(min = 1, max = 100))]
    jpg_quality: u8,
    /// 투명 부분 배경색 (JPG)
    #[schemars(extend("format" = "color"))]
    background: String,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            format: OutFmt::Png,
            jpg_quality: 90,
            background: "#ffffff".into(),
            metadata: MetaMode::Strip,
        }
    }
}

struct Convert;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Convert)
}

fn convert(
    input: &Path,
    out_dir: &Path,
    o: &ConvertOptions,
    bg: [u8; 3],
) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (fmt, img) = match o.format {
        OutFmt::Jpg => (Fmt::Jpeg, flatten(&src.image, bg)),
        OutFmt::Png => (Fmt::Png, src.image),
        OutFmt::Webp => (Fmt::Webp, src.image),
    };
    let q = Quality {
        quality: if fmt == Fmt::Jpeg { o.jpg_quality } else { 90 },
        lossless: false,
    };
    let mut bytes = encode(&img, fmt, q)?;
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, fmt, &src.meta)?;
    }
    write_output(out_dir, input, fmt, &bytes)
}

impl Tool for Convert {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.convert".into(),
            name: "포맷 변환".into(),
            category: "image".into(),
            description: "JPG·PNG·WebP 사이에서 변환합니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<ConvertOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: ConvertOptions = parse_options(opts)?;
        let bg = parse_color(&o.background).ok_or_else(|| {
            ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.".into())
        })?;
        run_each(ctx, input, |p| convert(p, out_dir, &o, bg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, ToolError, ToolOutput};
    use localtools_image_common::testkit::{alpha_png, photo};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn run(input: &Path, out: &Path, opts: serde_json::Value) -> Result<PathBuf, ToolError> {
        let ctx = JobContext::new(|_| {}, Default::default());
        match tool().run(&ctx, &[input.to_path_buf()], out, &opts)? {
            ToolOutput::Files(f) => Ok(f[0].clone()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn png_to_jpg_changes_extension() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(16, 16).save(&src).unwrap();
        let name = run(&src, d.path(), json!({ "format": "jpg" })).unwrap();
        assert_eq!(name, Path::new("a.jpg"));
        let bytes = std::fs::read(d.path().join(name)).unwrap();
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::Jpeg
        );
    }

    #[test]
    fn transparent_png_to_jpg_uses_background() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        alpha_png(&src, 32, 32);
        let name = run(
            &src,
            d.path(),
            json!({ "format": "jpg", "background": "#ff0000" }),
        )
        .unwrap();
        let px = image::open(d.path().join(name))
            .unwrap()
            .to_rgb8()
            .get_pixel(2, 2)
            .0;
        assert!(px[0] > 245 && px[1] < 10 && px[2] < 10, "{px:?}");
    }

    #[test]
    fn jpg_to_png_is_png() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        photo(16, 16).save(&src).unwrap();
        let name = run(&src, d.path(), json!({ "format": "png" })).unwrap();
        assert_eq!(name, Path::new("a.png"));
    }

    #[test]
    fn bad_background_is_invalid_options() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(4, 4).save(&src).unwrap();
        let r = run(
            &src,
            d.path(),
            json!({ "format": "jpg", "background": "red" }),
        );
        assert!(
            matches!(r, Err(ToolError::InvalidOptions(m)) if m == "배경색은 #rrggbb 형식이어야 합니다.")
        );
    }

    #[test]
    fn schema_marks_background_as_color() {
        let s = tool().options_schema();
        assert_eq!(s["properties"]["background"]["format"], "color");
        assert_eq!(tool().meta().name, "포맷 변환");
    }
}
