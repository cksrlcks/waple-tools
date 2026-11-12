//! 업스케일 (추가 설치, PerFile): Real-ESRGAN x4plus로 2배·4배 키운다. 알파는 일반 리사이즈로 따로 키운다.

#[cfg(test)]
mod tests;
mod tile;

use std::path::{Path, PathBuf};

use image::imageops::{FilterType, resize};
use image::{DynamicImage, RgbaImage};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_ai_common::{Device, Model, Runner};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, apply_meta, encode, ensure_pixels, load, parse_options,
    rgb_icc_only, schema_of, write_output,
};

#[cfg(has_model)]
const BYTES: &[u8] = include_bytes!("../model/model.onnx");
#[cfg(not(has_model))]
const BYTES: &[u8] = &[];
/// Real-ESRGAN x4plus: `[1, 3, 128, 128]` → `[1, 3, 512, 512]` (0~1 RGB)
static MODEL: Model = Model::new(
    BYTES,
    [1, 3, tile::TILE as usize, tile::TILE as usize],
    Device::Gpu,
);

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
enum Scale {
    /// 2배
    X2,
    /// 4배
    #[default]
    X4,
}

impl Scale {
    fn factor(self) -> u32 {
        match self {
            Scale::X2 => 2,
            Scale::X4 => 4,
        }
    }
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    // 키우는 도구라 결과가 늘 원본보다 크다
    "x-allow-larger" = true,
    // 큰 이미지는 수십 초~수 분 걸린다: 한 파일씩, 미리보기는 가운데 256×256 조각만
    "x-single-input" = true,
    "x-preview-crop" = 256
))]
struct UpscaleOptions {
    /// 배율
    ///
    /// 2배는 4배로 키운 뒤 고품질로 줄입니다. 큰 이미지는 몇 분 걸릴 수 있습니다.
    scale: Scale,
}

/// 결과 한 변 상한
pub const MAX_SIDE: u32 = 16384;
/// 다시 저장할 때 화질 손실을 줄이는 JPEG·WebP 품질
const QUALITY: u8 = 92;

/// 결과 크기. 긴 변·화소 수 상한을 넘으면 실패
fn out_size(w: u32, h: u32, scale: u32) -> Result<(u32, u32), ToolError> {
    let (ow, oh) = (
        u64::from(w) * u64::from(scale),
        u64::from(h) * u64::from(scale),
    );
    if ow.max(oh) > u64::from(MAX_SIDE) {
        return Err(ToolError::Failed(format!(
            "결과 이미지가 너무 큽니다 (긴 변 {MAX_SIDE}px 초과). 배율을 낮추거나 더 작은 이미지를 써 주세요."
        )));
    }
    let (ow, oh) = (ow as u32, oh as u32);
    ensure_pixels(ow, oh)?;
    Ok((ow, oh))
}

fn upscale(
    r: &mut Runner,
    ctx: &JobContext,
    input: &Path,
    out_dir: &Path,
    scale: u32,
    progress: impl Fn(f32),
) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (w, h) = (src.image.width(), src.image.height());
    let (ow, oh) = out_size(w, h, scale)?;
    let rgb = tile::upscale_rgb(
        &src.image.to_rgb8(),
        scale,
        |t| {
            ctx.check_cancel()?;
            r.infer(t.to_vec())
        },
        |done, total| {
            progress(done as f32 / total as f32);
            ctx.check_cancel()
        },
    )?;
    let (out, fmt) = if src.image.color().has_alpha() {
        let alpha = resize(&src.image.to_luma_alpha8(), ow, oh, FilterType::Lanczos3);
        let rgba = RgbaImage::from_fn(ow, oh, |x, y| {
            let [r, g, b] = rgb.get_pixel(x, y).0;
            image::Rgba([r, g, b, alpha.get_pixel(x, y)[1]])
        });
        (DynamicImage::ImageRgba8(rgba), Fmt::Png)
    } else {
        (DynamicImage::ImageRgb8(rgb), src.format)
    };
    let bytes = encode(
        &out,
        fmt,
        Quality {
            quality: QUALITY,
            lossless: false,
        },
    )?;
    // 지우기: EXIF는 빼고 RGB 색상 프로파일(ICC)만 남긴다
    let bytes = apply_meta(bytes, fmt, &rgb_icc_only(&src.meta), MetaMode::Strip)?;
    write_output(out_dir, input, fmt, &bytes)
}

struct ImageUpscale;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageUpscale)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for ImageUpscale {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.upscale".into(),
            name: "업스케일".into(),
            category: "image".into(),
            description: "AI로 이미지를 2배·4배 선명하게 키웁니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<UpscaleOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: UpscaleOptions = parse_options(opts)?;
        let n = input.len().max(1) as f32;
        let mut names = Vec::with_capacity(input.len());
        for (i, p) in input.iter().enumerate() {
            ctx.check_cancel()?;
            // 파일 하나를 처리하는 동안 세션을 잡는다: 병렬 작업이 와도 디코딩·결과 버퍼가 하나씩만 뜬다
            let name = MODEL.with_session(|r| {
                // 기다리는 사이 취소됐을 수 있다
                ctx.check_cancel()?;
                upscale(r, ctx, p, out_dir, o.scale.factor(), |x| {
                    ctx.progress((i as f32 + x) / n)
                })
            })?;
            names.push(name);
        }
        Ok(ToolOutput::Files(names))
    }
}
