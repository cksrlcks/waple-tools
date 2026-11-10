//! 이미지 최적화 (기본 내장): 크기 → 포맷 → 용량, 단계마다 스위치로 켜고 끈다

mod quality;
mod size;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, apply_meta, encode, flatten, load, parse_color,
    parse_options, run_each, schema_of, write_output,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::quality::{encode_png, search_quality};
use crate::size::resize_image;

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SizeMode {
    /// 가로 기준
    Width,
    /// 세로 기준
    Height,
    /// 긴 변 기준
    #[default]
    LongEdge,
    /// 정확한 크기
    Exact,
    /// SNS 규격
    Preset,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Preset {
    /// OG 1200×630
    #[default]
    Og,
    /// X(트위터) 1600×900
    X,
    /// 인스타 정사각 1080×1080
    InstaSquare,
    /// 인스타 세로 1080×1350
    InstaPortrait,
    /// 스토리 1080×1920
    Story,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Fit {
    /// 비율 유지 (안에 맞춤)
    Contain,
    /// 늘리기
    Stretch,
    /// 크롭해서 채우기
    #[default]
    Cover,
    /// 여백 추가
    Pad,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum OutFmt {
    /// JPG
    Jpg,
    /// PNG
    Png,
    /// WebP
    #[default]
    Webp,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum QualityMode {
    /// 품질 지정
    #[default]
    Quality,
    /// 목표 용량
    TargetKb,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-require-any" = ["resize", "convert", "compress"]))]
struct OptimizeOptions {
    /// 크기 바꾸기
    #[schemars(extend("x-group" = "크기", "x-group-toggle" = true))]
    resize: bool,
    /// 기준
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true] }))]
    size: SizeMode,
    /// 가로(px) — 긴 변 기준일 때는 긴 변 길이
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true], "size": ["width", "longEdge", "exact"] }))]
    width: u32,
    /// 세로(px)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true], "size": ["height", "exact"] }))]
    height: u32,
    /// 규격
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true], "size": ["preset"] }))]
    preset: Preset,
    /// 맞춤
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true], "size": ["exact", "preset"] }))]
    fit: Fit,
    /// 확대 금지
    #[schemars(extend("x-group" = "크기", "x-enabled-when" = { "resize": [true] }))]
    no_upscale: bool,

    /// 포맷 바꾸기
    #[schemars(extend("x-group" = "포맷", "x-group-toggle" = true))]
    convert: bool,
    /// 출력 포맷
    #[schemars(extend("x-group" = "포맷", "x-enabled-when" = { "convert": [true] }))]
    format: OutFmt,
    /// 무손실 (WebP)
    #[schemars(extend("x-group" = "포맷", "x-enabled-when" = { "convert": [true], "format": ["webp"] }))]
    lossless: bool,
    /// 투명 부분·여백 배경색
    #[schemars(extend("format" = "color", "x-group" = "포맷",
        "x-enabled-when" = [{ "convert": [true], "format": ["jpg"] }, { "resize": [true], "fit": ["pad"] }]))]
    background: String,

    /// 용량 줄이기
    #[schemars(extend("x-group" = "용량", "x-group-toggle" = true))]
    compress: bool,
    /// 방식
    #[schemars(extend("x-group" = "용량", "x-enabled-when" = { "compress": [true] }))]
    mode: QualityMode,
    /// 품질
    #[schemars(range(min = 1, max = 100))]
    #[schemars(extend("x-group" = "용량", "x-enabled-when" = { "compress": [true], "mode": ["quality"] }))]
    quality: u8,
    /// 목표 용량(KB)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "용량", "x-enabled-when" = { "compress": [true], "mode": ["targetKb"] }))]
    target_kb: u32,
    /// PNG 색상 수 (0은 줄이지 않음, 손실)
    #[schemars(range(min = 0, max = 256))]
    #[schemars(extend("x-group" = "용량", "x-enabled-when" = { "compress": [true] }))]
    png_colors: u16,

    /// 메타데이터
    #[schemars(extend("x-group" = "저장"))]
    metadata: MetaMode,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            resize: false,
            size: SizeMode::LongEdge,
            width: 1920,
            height: 1080,
            preset: Preset::Og,
            fit: Fit::Cover,
            no_upscale: true,
            convert: false,
            format: OutFmt::Webp,
            lossless: false,
            background: "#ffffff".into(),
            compress: true,
            mode: QualityMode::Quality,
            quality: 80,
            target_kb: 200,
            png_colors: 0,
            metadata: MetaMode::Strip,
        }
    }
}

/// 용량 단계가 꺼져 있을 때 화질을 최대한 유지하는 품질
const HIGH_QUALITY: u8 = 95;

fn optimize(
    input: &Path,
    out_dir: &Path,
    o: &OptimizeOptions,
    bg: [u8; 3],
) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let fmt = if o.convert {
        match o.format {
            OutFmt::Jpg => Fmt::Jpeg,
            OutFmt::Png => Fmt::Png,
            OutFmt::Webp => Fmt::Webp,
        }
    } else {
        match src.format {
            // GIF·BMP·TIFF는 압축할 수 없어 PNG로 저장
            Fmt::Gif | Fmt::Bmp | Fmt::Tiff => Fmt::Png,
            f => f,
        }
    };
    let transparent = matches!(fmt, Fmt::Png | Fmt::Webp);
    let img = resize_image(src.image, o, transparent, bg)?;
    let img = if fmt == Fmt::Jpeg && img.color().has_alpha() {
        flatten(&img, bg)
    } else {
        img
    };

    let lossy = |q: u8| {
        encode(
            &img,
            fmt,
            Quality {
                quality: q,
                lossless: false,
            },
        )
    };
    let mut bytes = match fmt {
        Fmt::Png if o.compress => encode_png(&img, o.png_colors, 4)?,
        Fmt::Png => encode_png(&img, 0, 2)?,
        Fmt::Webp if o.convert && o.lossless => encode(
            &img,
            fmt,
            Quality {
                quality: 100,
                lossless: true,
            },
        )?,
        _ if !o.compress => lossy(HIGH_QUALITY)?,
        _ => match o.mode {
            QualityMode::Quality => lossy(o.quality.clamp(1, 100))?,
            QualityMode::TargetKb => search_quality(o.target_kb.max(1) as usize * 1024, lossy)?,
        },
    };
    bytes = apply_meta(bytes, fmt, &src.meta, o.metadata)?;
    write_output(out_dir, input, fmt, &bytes)
}

struct Optimize;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Optimize)
}

impl Tool for Optimize {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.optimize".into(),
            name: "이미지 최적화".into(),
            category: "image".into(),
            description: "크기·포맷·품질을 한 번에 맞춥니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<OptimizeOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: OptimizeOptions = parse_options(opts)?;
        if !(o.resize || o.convert || o.compress) {
            return Err(ToolError::InvalidOptions(
                "적용할 단계를 하나 이상 켜세요.".into(),
            ));
        }
        let bg = parse_color(&o.background).ok_or_else(|| {
            ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.".into())
        })?;
        run_each(ctx, input, |p| optimize(p, out_dir, &o, bg))
    }
}
