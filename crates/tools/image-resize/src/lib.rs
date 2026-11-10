//! 리사이즈 (기본 내장)

use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, encode, flatten, load, parse_options, run_each, schema_of,
    with_meta, write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

/// 한 변의 최대 크기
const MAX_SIDE: u32 = 20_000;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 가로 기준
    Width,
    /// 세로 기준
    Height,
    /// 긴 변 기준
    #[default]
    LongEdge,
    /// 정확한 크기
    Exact,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Fit {
    /// 비율 유지 (안에 맞춤)
    #[default]
    Contain,
    /// 늘리기
    Stretch,
    /// 크롭해서 채우기
    Cover,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum OutFmt {
    /// 원본 유지
    #[default]
    Keep,
    /// JPG
    Jpg,
    /// PNG
    Png,
    /// WebP
    Webp,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct ResizeOptions {
    /// 기준
    mode: Mode,
    /// 가로(px) — 긴 변 기준일 때는 긴 변 길이
    #[schemars(range(min = 1))]
    width: u32,
    /// 세로(px)
    #[schemars(range(min = 1))]
    height: u32,
    /// 맞춤 (정확한 크기일 때)
    fit: Fit,
    /// 확대 금지
    no_upscale: bool,
    /// 출력 포맷
    format: OutFmt,
    /// 품질 (JPG·WebP)
    #[schemars(range(min = 1, max = 100))]
    quality: u8,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            mode: Mode::LongEdge,
            width: 1920,
            height: 1080,
            fit: Fit::Contain,
            no_upscale: true,
            format: OutFmt::Keep,
            quality: 90,
            metadata: MetaMode::Strip,
        }
    }
}

/// 비율 유지 환산: `n * to / from` (최소 1)
fn scale(n: u32, from: u32, to: u32) -> u32 {
    ((f64::from(n) * f64::from(to) / f64::from(from.max(1))).round() as u32).max(1)
}

/// 결과 크기. 확대 금지면 어느 한 변이라도 커질 때 원본 크기를 쓴다.
fn target_size(w: u32, h: u32, o: &ResizeOptions) -> (u32, u32) {
    let bw = o.width.clamp(1, MAX_SIDE);
    let bh = o.height.clamp(1, MAX_SIDE);
    let (tw, th) = match o.mode {
        Mode::Width => (bw, scale(h, w, bw)),
        Mode::Height => (scale(w, h, bh), bh),
        Mode::LongEdge if w >= h => (bw, scale(h, w, bw)),
        Mode::LongEdge => (scale(w, h, bw), bw),
        Mode::Exact => match o.fit {
            Fit::Stretch | Fit::Cover => (bw, bh),
            Fit::Contain => {
                let r = (f64::from(bw) / f64::from(w)).min(f64::from(bh) / f64::from(h));
                (
                    ((f64::from(w) * r).round() as u32).max(1),
                    ((f64::from(h) * r).round() as u32).max(1),
                )
            }
        },
    };
    let (tw, th) = (tw.min(MAX_SIDE), th.min(MAX_SIDE));
    if o.no_upscale && (tw > w || th > h) {
        (w, h)
    } else {
        (tw, th)
    }
}

struct Resize;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Resize)
}

fn resize(input: &Path, out_dir: &Path, o: &ResizeOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (w, h) = (src.image.width(), src.image.height());
    let (tw, th) = target_size(w, h, o);
    let img = if (tw, th) == (w, h) {
        src.image
    } else if o.mode == Mode::Exact && o.fit == Fit::Cover {
        src.image.resize_to_fill(tw, th, FilterType::Lanczos3)
    } else {
        src.image.resize_exact(tw, th, FilterType::Lanczos3)
    };
    let fmt = match o.format {
        OutFmt::Keep => src.format,
        OutFmt::Jpg => Fmt::Jpeg,
        OutFmt::Png => Fmt::Png,
        OutFmt::Webp => Fmt::Webp,
    };
    let img = if fmt == Fmt::Jpeg && img.color().has_alpha() {
        flatten(&img, [255, 255, 255])
    } else {
        img
    };
    let q = Quality {
        quality: o.quality.clamp(1, 100),
        lossless: false,
    };
    let mut bytes = encode(&img, fmt, q)?;
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, fmt, &src.meta)?;
    }
    write_output(out_dir, input, fmt, &bytes)
}

impl Tool for Resize {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.resize".into(),
            name: "리사이즈".into(),
            category: "image".into(),
            description: "가로·세로·긴 변 기준 또는 정확한 크기로 줄입니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<ResizeOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: ResizeOptions = parse_options(opts)?;
        run_each(ctx, input, |p| resize(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, ToolOutput};
    use localtools_image_common::testkit::photo;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn opts(v: serde_json::Value) -> ResizeOptions {
        localtools_image_common::parse_options(&v).unwrap()
    }

    #[test]
    fn target_size_table() {
        let t = |w, h, v| target_size(w, h, &opts(v));
        assert_eq!(
            t(4000, 3000, json!({"mode": "width", "width": 1920})),
            (1920, 1440)
        );
        assert_eq!(
            t(4000, 3000, json!({"mode": "height", "height": 1080})),
            (1440, 1080)
        );
        assert_eq!(
            t(3000, 4000, json!({"mode": "longEdge", "width": 1920})),
            (1440, 1920)
        );
        assert_eq!(
            t(
                4000,
                3000,
                json!({"mode": "exact", "width": 1000, "height": 1000, "fit": "contain"})
            ),
            (1000, 750)
        );
        assert_eq!(
            t(
                4000,
                3000,
                json!({"mode": "exact", "width": 1000, "height": 1000, "fit": "stretch"})
            ),
            (1000, 1000)
        );
        assert_eq!(
            t(
                4000,
                3000,
                json!({"mode": "exact", "width": 1000, "height": 1000, "fit": "cover"})
            ),
            (1000, 1000)
        );
    }

    #[test]
    fn no_upscale_keeps_size() {
        let t = |w, h, v| target_size(w, h, &opts(v));
        assert_eq!(
            t(800, 600, json!({"mode": "width", "width": 1920})),
            (800, 600)
        );
        assert_eq!(
            t(
                800,
                600,
                json!({"mode": "exact", "width": 1000, "height": 1000, "fit": "stretch"})
            ),
            (800, 600)
        );
        assert_eq!(
            t(
                800,
                600,
                json!({"mode": "width", "width": 1920, "noUpscale": false})
            ),
            (1920, 1440)
        );
    }

    #[test]
    fn huge_size_is_capped() {
        assert_eq!(
            target_size(
                100,
                100,
                &opts(json!({"mode": "width", "width": 999999, "noUpscale": false}))
            ),
            (20000, 20000)
        );
    }

    fn run(input: &Path, out: &Path, v: serde_json::Value) -> PathBuf {
        let ctx = JobContext::new(|_| {}, Default::default());
        match tool().run(&ctx, &[input.to_path_buf()], out, &v).unwrap() {
            ToolOutput::Files(f) => f[0].clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cover_crops_to_exact_box() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(400, 300).save(&src).unwrap();
        let name = run(
            &src,
            d.path(),
            json!({"mode": "exact", "width": 200, "height": 200, "fit": "cover"}),
        );
        let img = image::open(d.path().join(name)).unwrap();
        assert_eq!((img.width(), img.height()), (200, 200));
    }

    #[test]
    fn keep_format_keeps_extension() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(400, 300).save(&src).unwrap();
        let name = run(&src, d.path(), json!({"mode": "width", "width": 100}));
        assert_eq!(name, Path::new("a.png"));
        let img = image::open(d.path().join(name)).unwrap();
        assert_eq!((img.width(), img.height()), (100, 75));
    }

    #[test]
    fn change_format_to_webp() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(40, 30).save(&src).unwrap();
        assert_eq!(
            run(&src, d.path(), json!({"width": 20, "format": "webp"})),
            Path::new("a.webp")
        );
        assert_eq!(tool().meta().name, "리사이즈");
    }
}
