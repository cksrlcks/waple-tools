//! 배경 제거 (추가 설치, PerFile): BiRefNet_lite 마스크를 원본 크기로 올려 알파로 적용하고 `이름.png`로 저장한다.

mod mask;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use image::DynamicImage;
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_ai_common::{Device, Model, Runner};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, apply_meta, encode, flatten, load, parse_color,
    parse_options, rgb_icc_only, run_each, schema_of, write_output,
};

#[cfg(has_model)]
const BYTES: &[u8] = include_bytes!("../model/model.onnx");
#[cfg(not(has_model))]
const BYTES: &[u8] = &[];
/// BiRefNet_lite: `[1, 3, 1024, 1024]` (ImageNet 정규화) → 로짓 `[1, 1, 1024, 1024]`
// 늘 CPU: 내장 GPU에서는 디폼 합성곱 중간값이 커 DirectML이 메모리 부족·장치 리셋을 낸다
static MODEL: Model = Model::new(
    BYTES,
    [1, 3, mask::SIDE as usize, mask::SIDE as usize],
    Device::Cpu,
);

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
enum Background {
    /// 투명
    #[default]
    Transparent,
    /// 단색
    Color,
    /// 마스크만 (흑백)
    Mask,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    // JPEG를 투명 PNG로 바꾸므로 커질 수 있다
    "x-allow-larger" = true,
    // 모델 실행이 느려 한 파일씩, 미리보기는 도구를 실행하지 않고 원본만
    "x-single-input" = true,
    "x-preview" = "original"
))]
struct BgRemoveOptions {
    /// 배경
    ///
    /// 마스크만은 남길 부분을 흰색, 지울 부분을 검정으로 저장합니다.
    background: Background,
    /// 배경색
    #[schemars(extend("format" = "color", "x-enabled-when" = { "background": ["color"] }))]
    color: String,
}

impl Default for BgRemoveOptions {
    fn default() -> Self {
        Self {
            background: Background::Transparent,
            color: "#ffffff".into(),
        }
    }
}

fn remove_bg(
    r: &mut Runner,
    input: &Path,
    out_dir: &Path,
    background: Background,
    color: [u8; 3],
) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (w, h) = (src.image.width(), src.image.height());
    let logits = r.infer(mask::to_input(&src.image.to_rgb8()))?;
    let m = mask::to_mask(&logits, w, h)?;
    let out = match background {
        Background::Transparent => {
            DynamicImage::ImageRgba8(mask::cut_out(&src.image.to_rgba8(), &m))
        }
        Background::Color => flatten(
            &DynamicImage::ImageRgba8(mask::cut_out(&src.image.to_rgba8(), &m)),
            color,
        ),
        Background::Mask => DynamicImage::ImageLuma8(m),
    };
    let bytes = encode(
        &out,
        Fmt::Png,
        Quality {
            quality: 100,
            lossless: true,
        },
    )?;
    // 색상 프로파일(ICC)은 남긴다. 흑백 마스크에는 RGB 프로파일을 붙이지 않는다
    let bytes = if background == Background::Mask {
        bytes
    } else {
        apply_meta(bytes, Fmt::Png, &rgb_icc_only(&src.meta), MetaMode::Strip)?
    };
    write_output(out_dir, input, Fmt::Png, &bytes)
}

struct ImageBgRemove;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageBgRemove)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for ImageBgRemove {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.bg_remove".into(),
            name: "배경 제거".into(),
            category: "image".into(),
            description: "AI로 사람·사물만 남기고 배경을 지웁니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<BgRemoveOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: BgRemoveOptions = parse_options(opts)?;
        let color = parse_color(&o.color).ok_or_else(|| {
            ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.".into())
        })?;
        run_each(ctx, input, |p| {
            // 파일 하나를 처리하는 동안 세션을 잡는다: 병렬 작업이 와도 디코딩·추론 버퍼가 하나씩만 뜬다
            MODEL.with_session(|r| {
                // 기다리는 사이 취소됐을 수 있다
                ctx.check_cancel()?;
                remove_bg(r, p, out_dir, o.background, color)
            })
        })
    }
}
