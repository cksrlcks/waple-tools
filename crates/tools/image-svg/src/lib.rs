//! SVG 도구 (추가 설치, PerFile): 최적화(.svg) 또는 PNG로 내보내기(.png)

mod numbers;
mod optimize;
mod render;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{parse_color, parse_options, run_each, schema_of};

use crate::optimize::Clean;
use crate::render::Size;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 최적화 (SVG)
    #[default]
    Optimize,
    /// PNG로 내보내기
    Png,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SizeMode {
    /// 배율
    #[default]
    Scale,
    /// 가로(px)
    Width,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Background {
    /// 투명
    #[default]
    Transparent,
    /// 단색
    Solid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    // 썸네일·비교 미리보기는 SVG를 그릴 수 없다
    "x-preview" = "none",
    // PNG로 내보내면 거의 늘 원본보다 커진다: 그때만 "결과가 크면 원본 유지" 설정을 쓰지 않는다
    "x-allow-larger-when" = { "mode": ["png"] }
))]
struct SvgOptions {
    /// 모드
    ///
    /// 최적화: 보이는 모양은 그대로 두고 필요 없는 내용을 지워 용량을 줄입니다. XML 선언·DOCTYPE과 요소 사이 공백은 늘 지우고, viewBox는 남깁니다 (없으면 width·height로 만듭니다). PNG로 내보내기: 정한 크기의 PNG 이미지로 바꿉니다.
    mode: Mode,

    /// 소수점 자릿수
    ///
    /// 좌표·크기 숫자(path, points, transform, x·y·width·height 등)를 이 자릿수로 반올림합니다. 작을수록 용량이 줄지만 모양이 조금 달라질 수 있습니다.
    #[schemars(range(min = 0, max = 8))]
    #[schemars(extend(
        "x-group" = "최적화",
        "x-enabled-when" = { "mode": ["optimize"] }
    ))]
    precision: u8,
    /// 주석·메타데이터 제거
    ///
    /// 주석, `<metadata>`, 편집기(Inkscape·Illustrator·Sketch·Figma 등)가 남긴 전용 요소·속성을 지웁니다. `<!--!`로 시작하는 저작권 주석은 남깁니다.
    #[schemars(extend("x-group" = "최적화", "x-enabled-when" = { "mode": ["optimize"] }))]
    strip_metadata: bool,
    /// 빈 그룹 제거
    ///
    /// 내용이 없는 `<g>`, `<defs>`, `<symbol>` 등을 지웁니다. 필터가 걸린 그룹처럼 비어도 그려지는 것은 남깁니다.
    #[schemars(extend("x-group" = "최적화", "x-enabled-when" = { "mode": ["optimize"] }))]
    remove_empty: bool,

    /// 크기 기준
    #[schemars(extend("x-group" = "PNG", "x-enabled-when" = { "mode": ["png"] }))]
    size_mode: SizeMode,
    /// 배율
    ///
    /// SVG 원래 크기(width·height, 없으면 viewBox)의 몇 배로 그릴지 정합니다. 2면 고해상도 화면용 2x입니다.
    #[schemars(range(min = 0.1, max = 10))]
    #[schemars(extend("x-group" = "PNG", "x-enabled-when" = { "mode": ["png"], "sizeMode": ["scale"] }))]
    scale: f32,
    /// 가로(px)
    ///
    /// 세로는 비율대로 정해집니다.
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "PNG", "x-enabled-when" = { "mode": ["png"], "sizeMode": ["width"] }))]
    width: u32,
    /// 배경
    #[schemars(extend("x-group" = "PNG", "x-enabled-when" = { "mode": ["png"] }))]
    background: Background,
    /// 배경색
    #[schemars(extend(
        "format" = "color",
        "x-group" = "PNG",
        "x-enabled-when" = { "mode": ["png"], "background": ["solid"] }
    ))]
    color: String,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Optimize,
            precision: 3,
            strip_metadata: true,
            remove_empty: true,
            size_mode: SizeMode::Scale,
            scale: 1.0,
            width: 512,
            background: Background::Transparent,
            color: "#ffffff".into(),
        }
    }
}

struct ImageSvg;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageSvg)
}

waple_tools_addon_abi::export_tool!(tool);

/// 결과 내용과 확장자
fn convert(bytes: &[u8], o: &SvgOptions) -> Result<(Vec<u8>, &'static str), ToolError> {
    match o.mode {
        Mode::Optimize => {
            let clean = Clean {
                precision: o.precision.min(8),
                strip_meta: o.strip_metadata,
                remove_empty: o.remove_empty,
            };
            Ok((optimize::optimize(bytes, &clean)?.into_bytes(), "svg"))
        }
        Mode::Png => {
            let size = match o.size_mode {
                SizeMode::Scale if o.scale.is_finite() && o.scale > 0.0 => Size::Scale(o.scale),
                SizeMode::Width if o.width > 0 => Size::Width(o.width),
                _ => {
                    return Err(ToolError::InvalidOptions(
                        "배율·가로 크기는 0보다 커야 합니다.".into(),
                    ));
                }
            };
            let bg = match o.background {
                Background::Transparent => None,
                Background::Solid => Some(parse_color(&o.color).ok_or_else(|| {
                    ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.".into())
                })?),
            };
            let tree = render::parse(bytes)?;
            Ok((render::render_png(&tree, size, bg)?, "png"))
        }
    }
}

impl Tool for ImageSvg {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.svg".into(),
            name: "SVG 도구".into(),
            category: "image".into(),
            description: "SVG를 최적화하거나 PNG로 내보냅니다.".into(),
            extensions: vec!["svg".into()],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<SvgOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: SvgOptions = parse_options(opts)?;
        run_each(ctx, input, |src| {
            let (out, ext) = convert(&std::fs::read(src)?, &o)?;
            let stem = src.file_stem().unwrap_or_default().to_string_lossy();
            let rel = PathBuf::from(format!("{stem}.{ext}"));
            std::fs::write(out_dir.join(&rel), out)?;
            Ok(rel)
        })
    }
}
