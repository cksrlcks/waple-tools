//! GIF → 영상 (추가 설치, PerFile): 움직이는 GIF를 H.264 mp4로. 프레임마다 다른 지연을 그대로 쓰고, 투명한 곳은 배경색으로 채운다.

#[cfg(test)]
mod tests;

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::gif::GifDecoder;
use image::imageops::{FilterType, resize};
use image::{AnimationDecoder, ImageDecoder, Rgba, RgbaImage};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{parse_options, schema_of};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Background {
    /// 흰색
    #[default]
    White,
    /// 검정
    Black,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct FromGifOptions {
    /// 배경색
    ///
    /// 영상에는 투명이 없어 투명한 곳을 이 색으로 채웁니다.
    background: Background,
}

impl Default for FromGifOptions {
    fn default() -> Self {
        Self {
            background: Background::White,
        }
    }
}

/// 화소당 비트 수. GIF는 경계가 또렷해서 영상 용량 줄이기 '높음'(0.1)보다 넉넉히
const BPP: f64 = 0.2;
/// 인코더 비트 배분 기준 초당 프레임 (실제 시각은 GIF 지연대로)
const NOMINAL_FPS: u32 = 30;

fn unreadable() -> ToolError {
    ToolError::Failed("GIF를 읽을 수 없습니다.".into())
}

/// 결과 크기: H.264가 받는 최소 크기보다 작으면 정수배로 키우고(또렷하게), 홀수면 오른쪽·아래에 배경 1px을 덧댄다.
/// 반환: (배율, 가로, 세로)
fn layout(w: u32, h: u32) -> (u32, u32, u32) {
    let k = mf::MIN_ENCODE_SIDE.div_ceil(w.min(h).max(1)).max(1);
    let up = |v: u32| (v * k).div_ceil(2) * 2;
    (k, up(w), up(h))
}

/// 알파를 배경 위에 합성하고 `k`배로 키워 `ow`×`oh` 캔버스 왼쪽 위에 놓는다
fn compose(f: &RgbaImage, bg: [u8; 3], k: u32, ow: u32, oh: u32) -> RgbaImage {
    let mut flat = f.clone();
    for p in flat.pixels_mut() {
        let a = u16::from(p[3]);
        for (c, b) in p.0[..3].iter_mut().zip(bg) {
            *c = ((u16::from(*c) * a + u16::from(b) * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
    let flat = if k > 1 {
        resize(&flat, f.width() * k, f.height() * k, FilterType::Nearest)
    } else {
        flat
    };
    if (flat.width(), flat.height()) == (ow, oh) {
        return flat;
    }
    let mut canvas = RgbaImage::from_pixel(ow, oh, Rgba([bg[0], bg[1], bg[2], 255]));
    image::imageops::replace(&mut canvas, &flat, 0, 0);
    canvas
}

fn from_gif(
    bytes: &[u8],
    dst: &Path,
    o: &FromGifOptions,
    ctx: &JobContext,
) -> Result<(), ToolError> {
    if image::guess_format(bytes).ok() != Some(image::ImageFormat::Gif) {
        return Err(ToolError::Failed("GIF 파일이 아닙니다.".into()));
    }
    let decoder = || GifDecoder::new(Cursor::new(bytes)).map_err(|_| unreadable());
    // 진행률용 프레임 수 (GIF 풀기는 인코딩보다 훨씬 빠르다)
    let total = decoder()?.into_frames().count();
    if total < 2 {
        return Err(ToolError::Failed(
            "움직이지 않는 GIF입니다. 프레임이 2개 이상인 GIF를 넣어 주세요.".into(),
        ));
    }
    let dec = decoder()?;
    let (w, h) = dec.dimensions();
    let (k, ow, oh) = layout(w, h);
    if ow.max(oh) > mf::MAX_ENCODE_SIDE {
        return Err(ToolError::Failed(format!(
            "GIF가 너무 큽니다 ({w}×{h}). 가로·세로 {}px 이하만 영상으로 만들 수 있습니다.",
            mf::MAX_ENCODE_SIDE
        )));
    }
    let bg = match o.background {
        Background::White => [255, 255, 255],
        Background::Black => [0, 0, 0],
    };
    let bps = ((f64::from(ow) * f64::from(oh) * f64::from(NOMINAL_FPS) * BPP) as u32).max(300_000);
    let writer = mf::Mp4Writer::create(dst, ow, oh, (NOMINAL_FPS, 1), bps)?;
    let mut time = 0i64;
    for (i, f) in dec.into_frames().enumerate() {
        ctx.check_cancel()?;
        let f = f.map_err(|_| unreadable())?;
        let (num, den) = f.delay().numer_denom_ms();
        // 브라우저는 10ms 이하 지연을 100ms로 재생한다 (GIF 최적화와 같은 규칙)
        let delay_ms = match num / den.max(1) {
            0..=10 => 100,
            ms => ms,
        };
        let dur = i64::from(delay_ms) * 10_000;
        let img = compose(f.buffer(), bg, k, ow, oh);
        writer.write(img.as_raw(), time, dur)?;
        time += dur;
        ctx.progress((i + 1) as f32 / total as f32);
    }
    writer.finish()
}

struct VideoFromGif;

pub fn tool() -> Box<dyn Tool> {
    Box::new(VideoFromGif)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for VideoFromGif {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video.from_gif".into(),
            name: "GIF → 영상".into(),
            category: "video".into(),
            description: "움직이는 GIF를 MP4(H.264) 영상으로 바꿉니다.".into(),
            extensions: vec!["gif".into()],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<FromGifOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: FromGifOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("GIF 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let bytes = std::fs::read(src)?;
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let rel = PathBuf::from(format!("{stem}.mp4"));
        from_gif(&bytes, &out_dir.join(&rel), &o, ctx)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
