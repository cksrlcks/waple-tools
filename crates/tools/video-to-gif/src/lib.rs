//! 영상 → GIF (추가 설치, PerFile): 시작 시간부터 길이만큼, 초당 프레임·너비를 골라 움직이는 GIF로. 색은 모든 프레임 공용 256색.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{GifFrame, Repeat, encode_gif, parse_options, schema_of};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Fps {
    /// 초당 10프레임
    #[default]
    F10,
    /// 초당 15프레임
    F15,
    /// 초당 20프레임
    F20,
}

impl Fps {
    fn value(self) -> u32 {
        match self {
            Self::F10 => 10,
            Self::F15 => 15,
            Self::F20 => 20,
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Width {
    /// 원본 크기
    Original,
    /// 640px
    W640,
    /// 480px
    #[default]
    W480,
    /// 320px
    W320,
}

impl Width {
    fn px(self) -> Option<u32> {
        match self {
            Self::Original => None,
            Self::W640 => Some(640),
            Self::W480 => Some(480),
            Self::W320 => Some(320),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    // 원본보다 커질 수 있다 (형식을 바꾸는 것이라 '용량 증가'로 버리면 안 된다)
    "x-allow-larger" = true,
    "x-preview" = "none"
))]
struct ToGifOptions {
    /// 시작 시간(초)
    #[schemars(range(min = 0))]
    start: f32,
    /// 길이(초)
    ///
    /// 최대 30초입니다. 영상이 먼저 끝나면 거기까지만 만듭니다.
    #[schemars(range(min = 0.5, max = 30))]
    length: f32,
    /// 초당 프레임
    ///
    /// 높을수록 부드럽지만 파일이 커집니다.
    fps: Fps,
    /// 너비
    ///
    /// 원본보다 작을 때만 줄이고 키우지는 않습니다.
    width: Width,
    /// 반복 재생
    #[serde(rename = "loop")]
    looping: bool,
}

impl Default for ToGifOptions {
    fn default() -> Self {
        Self {
            start: 0.0,
            length: 5.0,
            fps: Fps::F10,
            width: Width::W480,
            looping: true,
        }
    }
}

const MAX_LENGTH: f32 = 30.0;
/// 남길 프레임 수 × 화소 상한 (RGBA로 400MB, GIF 최적화와 같은 값)
const MAX_TOTAL_PIXELS: u64 = 100_000_000;
/// 디코딩이 차지하는 진행률 몫 (나머지는 GIF 저장)
const DECODE_SHARE: f32 = 0.6;

/// `k`번째 프레임의 지연(ms). GIF는 10ms 단위라 누적 시각을 반올림해 나눠 재생 시간이 밀리지 않게 한다
fn delay_ms(k: u32, fps: u32) -> u32 {
    let at = |k: u32| (k * 100 + fps / 2) / fps * 10;
    at(k + 1) - at(k)
}

fn to_gif(src: &Path, o: &ToGifOptions, ctx: &JobContext) -> Result<Vec<u8>, ToolError> {
    let probe = mf::probe(src)?;
    let start = f64::from(o.start.max(0.0));
    let mut length = f64::from(o.length.clamp(0.1, MAX_LENGTH));
    // 길이를 모르는 영상(녹화 webm 등)은 끝까지 읽어 보고 남는 시간은 마지막 프레임이 이어받는다
    if let Some(total) = probe.secs() {
        if start >= total {
            return Err(ToolError::InvalidOptions(format!(
                "시작 시간({start:.1}초)이 영상 길이({total:.1}초)보다 깁니다."
            )));
        }
        length = length.min(total - start);
    }
    let fps = o.fps.value();
    let n = ((length * fps as f64).round() as u32).max(1);
    let (sw, sh) = probe.shown_size();
    let (w, h) = mf::fit_width(sw, sh, o.width.px());
    if u64::from(w) * u64::from(h) * u64::from(n) > MAX_TOTAL_PIXELS {
        return Err(ToolError::InvalidOptions(
            "결과 GIF가 너무 큽니다. 길이·초당 프레임·너비를 줄여 주세요.".into(),
        ));
    }

    let mut reader = mf::FrameReader::open(src, &probe, w, h)?;
    let start_t = (start * 1e7) as i64;
    if start_t > 0 {
        reader.seek(start_t as u64)?;
    }
    let target = |k: u32| start_t + i64::from(k) * 10_000_000 / i64::from(fps);
    let mut frames: Vec<GifFrame> = Vec::new();
    // 마지막으로 GIF에 넣은 영상 프레임 번호: 같은 프레임이 이어지면 그 GIF 프레임의 지연만 늘린다
    let mut pushed = None;
    let mut emit = |id: usize, f: &mf::Frame, k: u32, reader: &mf::FrameReader| {
        let d = delay_ms(k, fps);
        match frames.last_mut() {
            Some(last) if pushed == Some(id) => last.delay_ms += d,
            _ => {
                frames.push(GifFrame {
                    rgba: reader.image(f)?,
                    delay_ms: d,
                });
                pushed = Some(id);
            }
        }
        Ok::<_, ToolError>(())
    };
    // 지금 화면에 있는 프레임 (번호, 프레임)
    let mut shown: Option<(usize, mf::Frame)> = None;
    let mut k = 0;
    for id in 0.. {
        ctx.check_cancel()?;
        let Some(f) = reader.next_frame()? else { break };
        // 목표 시각이 이 프레임보다 앞이면 그때 화면에 있던 앞 프레임 (화면 녹화처럼 간격이 들쭉날쭉해도 맞다).
        // 시작 시간 앞(키프레임부터 읽은 것)은 목표가 없어 그림으로 바꾸지 않고 지나간다
        while k < n && target(k) < f.time {
            match &shown {
                Some((sid, sf)) => emit(*sid, sf, k, &reader)?,
                None => emit(id, &f, k, &reader)?,
            }
            k += 1;
        }
        shown = Some((id, f));
        ctx.progress(k as f32 / n as f32 * DECODE_SHARE);
        if k == n {
            break;
        }
    }
    // 남은 목표 시각(영상 끝까지)은 마지막 프레임. 길이를 모르는 영상이면 끝난 것이니 한 칸만
    let (sid, sf) =
        shown.ok_or_else(|| ToolError::Failed("영상에서 프레임을 읽지 못했습니다.".into()))?;
    let end = if probe.duration == 0 {
        (k + 1).min(n)
    } else {
        n
    };
    for k in k..end {
        emit(sid, &sf, k, &reader)?;
    }
    let repeat = if o.looping {
        Repeat::Infinite
    } else {
        Repeat::Finite(0)
    };
    encode_gif(&frames, w, h, 256, repeat, ctx, &|r| {
        ctx.progress(DECODE_SHARE + r * (1.0 - DECODE_SHARE))
    })
}

struct VideoToGif;

pub fn tool() -> Box<dyn Tool> {
    Box::new(VideoToGif)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for VideoToGif {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video.to_gif".into(),
            name: "영상 → GIF".into(),
            category: "video".into(),
            description: "영상의 일부를 움직이는 GIF로 만듭니다.".into(),
            extensions: mf::VIDEO_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<ToGifOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: ToGifOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("영상 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let gif = to_gif(src, &o, ctx)?;
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let rel = PathBuf::from(format!("{stem}.gif"));
        std::fs::write(out_dir.join(&rel), gif)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
