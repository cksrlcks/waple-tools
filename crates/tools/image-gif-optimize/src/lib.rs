//! GIF 최적화 (추가 설치, PerFile): 프레임 건너뛰기 · 리사이즈 · 공용 팔레트로 색 줄이기 · 바뀐 부분만 다시 그리기

mod encode;
mod palette;
#[cfg(test)]
mod tests;

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, ImageDecoder, RgbaImage};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    Fmt, ensure_pixels, parse_options, resize_rgba, run_each, schema_of, write_output,
};

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct GifOptions {
    /// 색상 수
    ///
    /// 모든 프레임이 함께 쓰는 색 개수입니다. 적을수록 작아지지만 색이 거칠어집니다. 256이면 투명용 한 칸을 빼고 255색입니다.
    #[schemars(range(min = 2, max = 256))]
    colors: u16,
    /// 프레임 건너뛰기 (N개마다 1개)
    ///
    /// 1이면 모두 남깁니다. 건너뛴 프레임의 시간은 남은 프레임에 더해 재생 시간은 그대로입니다.
    #[schemars(range(min = 1, max = 10))]
    frame_step: u8,
    /// 크기(%)
    #[schemars(range(min = 10, max = 100))]
    scale: u8,
}

impl Default for GifOptions {
    fn default() -> Self {
        Self {
            colors: 128,
            frame_step: 1,
            scale: 100,
        }
    }
}

/// 합성된 전체 크기 프레임
pub(crate) struct Frame {
    pub rgba: RgbaImage,
    pub delay_ms: u32,
}

/// 남길 프레임 수 × 화소 상한 (RGBA로 400MB). 긴 GIF를 통째로 메모리에 올리다 멈추지 않게 한다.
const MAX_TOTAL_PIXELS: u64 = 100_000_000;

fn unreadable() -> ToolError {
    ToolError::Failed("GIF를 읽을 수 없습니다.".into())
}

/// 프레임을 합성해 읽으며 건너뛰고 줄인다. 반환: (프레임, 결과 가로, 세로)
fn decode(
    bytes: &[u8],
    o: &GifOptions,
    ctx: &JobContext,
) -> Result<(Vec<Frame>, u32, u32), ToolError> {
    let dec = GifDecoder::new(Cursor::new(bytes)).map_err(|_| unreadable())?;
    let (w, h) = dec.dimensions();
    ensure_pixels(w, h)?;
    let scale = u32::from(o.scale.clamp(10, 100));
    let (ow, oh) = (
        ((w * scale + 50) / 100).max(1),
        ((h * scale + 50) / 100).max(1),
    );
    let step = usize::from(o.frame_step.max(1));
    let mut frames: Vec<Frame> = Vec::new();
    for (i, f) in dec.into_frames().enumerate() {
        ctx.check_cancel()?;
        let f = f.map_err(|_| unreadable())?;
        let (num, den) = f.delay().numer_denom_ms();
        // 브라우저는 10ms 이하 지연을 100ms로 재생한다: 합치기 전에 맞춰야 빨라지지 않는다
        let delay_ms = match num / den.max(1) {
            0..=10 => 100,
            ms => ms,
        };
        if i % step != 0 {
            // 건너뛴 프레임의 시간은 앞에 남긴 프레임이 이어받는다 (i == 0은 늘 남으므로 last가 있다)
            if let Some(last) = frames.last_mut() {
                last.delay_ms = last.delay_ms.saturating_add(delay_ms);
            }
            continue;
        }
        if u64::from(ow) * u64::from(oh) * (frames.len() as u64 + 1) > MAX_TOTAL_PIXELS {
            return Err(ToolError::Failed(
                "GIF가 너무 큽니다 (프레임 수 × 크기가 1억 화소 초과). 프레임 건너뛰기나 크기(%)를 줄여 주세요.".into(),
            ));
        }
        let rgba = f.into_buffer();
        let rgba = if (ow, oh) == (w, h) {
            rgba
        } else {
            resize_rgba(&rgba, ow, oh, FilterType::Triangle)
        };
        frames.push(Frame { rgba, delay_ms });
    }
    if frames.is_empty() {
        return Err(unreadable());
    }
    Ok((frames, ow, oh))
}

fn optimize(bytes: &[u8], o: &GifOptions, ctx: &JobContext) -> Result<Vec<u8>, ToolError> {
    if image::guess_format(bytes).ok() != Some(image::ImageFormat::Gif) {
        return Err(ToolError::Failed("GIF 파일이 아닙니다.".into()));
    }
    // image의 GifDecoder는 반복 횟수를 알려 주지 않아 gif로 머리만 읽는다
    let repeat = gif::Decoder::new(Cursor::new(bytes))
        .map(|d| d.repeat())
        .unwrap_or_default();
    let (frames, w, h) = decode(bytes, o, ctx)?;
    let pal = palette::build(&frames, usize::from(o.colors.clamp(2, 256)));
    encode::write(&frames, w, h, &pal, repeat, ctx)
}

struct ImageGifOptimize;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageGifOptimize)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for ImageGifOptimize {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.gif_optimize".into(),
            name: "GIF 최적화".into(),
            category: "image".into(),
            description: "색상 수와 프레임을 줄여 GIF 용량을 줄입니다.".into(),
            extensions: vec!["gif".into()],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<GifOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: GifOptions = parse_options(opts)?;
        run_each(ctx, input, |p| {
            let out = optimize(&std::fs::read(p)?, &o, ctx)?;
            write_output(out_dir, p, Fmt::Gif, &out)
        })
    }
}
