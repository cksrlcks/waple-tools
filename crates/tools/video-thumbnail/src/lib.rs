//! 영상 썸네일 추출 (추가 설치, Generator): 고른 시각의 프레임 하나 → `<이름>_00m12s345.jpg` (폴더 없이 원본 옆).
//! `x-probe: true`로 부르면 파일을 만들지 않고 길이·fps 블록을 돌려준다 (작업대 탐색 막대용).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use image::DynamicImage;
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{Fmt, Quality, encode, parse_options, schema_of};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Format {
    /// JPG
    #[default]
    Jpg,
    /// PNG
    Png,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Width {
    /// 원본 크기
    #[default]
    Original,
    /// 1280px
    W1280,
    /// 640px
    W640,
    /// 320px
    W320,
}

impl Width {
    fn px(self) -> Option<u32> {
        match self {
            Self::Original => None,
            Self::W1280 => Some(1280),
            Self::W640 => Some(640),
            Self::W320 => Some(320),
        }
    }
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-single-input" = true,
    "x-preview" = "result",
    // 작업대의 탐색 막대로 정하는 옵션 (길이·fps는 `x-probe`로 묻는다)
    "x-seek" = "time",
    // 영상 → 이미지라 원본과 용량을 견주지 않는다
    "x-allow-larger" = true
))]
struct ThumbOptions {
    /// 시각(초)
    #[schemars(range(min = 0))]
    time: f64,
    /// 포맷
    format: Format,
    /// 너비
    ///
    /// 원본보다 작을 때만 줄이고 키우지는 않습니다.
    width: Width,
}

const JPEG_QUALITY: u8 = 90;
/// 프레임 시각 비교 허용 오차 (100ns 단위, 1ms): `k/fps`초가 버림으로 바로 앞 프레임에 걸리지 않게
const SLACK: i64 = 10_000;

/// `00m12s`, 한 시간 넘으면 `1h02m03s`. `ms`면 뒤에 밀리초 (`00m01s500`)
fn label(secs: f64, ms: bool) -> String {
    let total_ms = (secs * 1000.0).round() as u64;
    let s = total_ms / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    let mut out = if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m:02}m{s:02}s")
    };
    if ms {
        out += &format!("{:03}", total_ms % 1000);
    }
    out
}

struct VideoThumbnail;

pub fn tool() -> Box<dyn Tool> {
    Box::new(VideoThumbnail)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for VideoThumbnail {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video.thumbnail".into(),
            name: "영상 썸네일 추출".into(),
            category: "video".into(),
            description: "영상에서 고른 프레임을 JPG·PNG 이미지로 저장합니다.".into(),
            extensions: mf::VIDEO_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Generator
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<ThumbOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("영상 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let probe = mf::probe(src)?;
        if opts["x-probe"] == true {
            return probe_blocks(&probe);
        }
        let o: ThumbOptions = parse_options(opts)?;
        let t = if o.time.is_finite() {
            o.time.max(0.0)
        } else {
            0.0
        };
        let (sw, sh) = probe.shown_size();
        let (w, h) = mf::fit_width(sw, sh, o.width.px());
        let mut reader = mf::FrameReader::open(src, &probe, w, h)?;
        let (fmt, ext) = match o.format {
            Format::Jpg => (Fmt::Jpeg, "jpg"),
            Format::Png => (Fmt::Png, "png"),
        };

        // 그 시각에 화면에 있던 프레임 = 시각이 그 이하인 마지막 프레임 (seek은 앞 키프레임으로 간다)
        let mut at = (t * 1e7).round() as i64 + SLACK;
        let mut seek = at;
        let duration = probe.duration as i64;
        let (n, d) = probe.fps;
        if duration > 0 && at >= duration {
            // 끝을 넘으면 마지막 프레임: 마지막 프레임 앞쪽으로 가서 끝까지 읽는다
            let frame = if n > 0 {
                10_000_000 * i64::from(d) / i64::from(n)
            } else {
                0
            };
            seek = (duration - frame).max(0);
            at = duration;
        }
        reader.seek(seek as u64)?;
        let mut shot = None;
        while let Some(f) = reader.next_frame()? {
            ctx.check_cancel()?;
            if f.time > at {
                shot = shot.or(Some(f));
                break;
            }
            shot = Some(f);
        }
        let f = shot.ok_or_else(|| {
            ToolError::Failed(format!("{}의 화면을 읽지 못했습니다.", label(t, false)))
        })?;
        let img = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(reader.image(&f)?).to_rgb8());
        let data = encode(
            &img,
            fmt,
            Quality {
                quality: JPEG_QUALITY,
                lossless: false,
            },
        )?;
        // 이름표는 고른 시각 (길이를 넘었으면 영상 길이)
        let shown_t = probe.secs().map_or(t, |total| t.min(total));
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let name = PathBuf::from(format!("{stem}_{}.{ext}", label(shown_t, true)));
        std::fs::write(out_dir.join(&name), data)?;
        ctx.progress(1.0);
        Ok(ToolOutput::Files(vec![name]))
    }
}

/// 탐색 막대용 정보: 길이(초)·fps
fn probe_blocks(probe: &mf::Probe) -> Result<ToolOutput, ToolError> {
    let secs = probe.secs().ok_or_else(|| {
        ToolError::Failed("영상 길이를 알 수 없어 탐색 막대를 쓸 수 없습니다.".into())
    })?;
    let (n, d) = probe.fps;
    Ok(ToolOutput::Blocks(vec![
        Block::Number {
            label: "duration".into(),
            value: secs,
            unit: Some("s".into()),
        },
        Block::Number {
            label: "fps".into(),
            value: f64::from(n) / f64::from(d.max(1)),
            unit: None,
        },
    ]))
}
