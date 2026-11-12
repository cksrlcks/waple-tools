//! 영상 용량 줄이기 (추가 설치, PerFile): Windows에 든 H.264·AAC 인코더로 mp4를 다시 만든다. 해상도·화질(또는 목표 용량)·소리 유지를 고른다.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Resolution {
    /// 원본 크기
    #[default]
    Original,
    /// 1080p
    P1080,
    /// 720p
    P720,
    /// 480p
    P480,
}

impl Resolution {
    /// 짧은 변 길이 (세로 영상도 같은 기준)
    fn short_side(self) -> Option<u32> {
        match self {
            Self::Original => None,
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Quality {
    /// 높음
    High,
    /// 보통
    #[default]
    Medium,
    /// 낮음 (가장 작게)
    Low,
    /// 목표 용량
    TargetMb,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct VideoOptions {
    /// 해상도
    ///
    /// 짧은 변 기준입니다. 원본보다 작을 때만 줄이고 키우지는 않습니다.
    resolution: Resolution,
    /// 화질
    ///
    /// 목표 용량: 파일이 이 크기에 맞도록 화질을 정합니다. 영상이 길수록 화질이 낮아집니다.
    quality: Quality,
    /// 목표 용량 (MB)
    #[schemars(range(min = 1, max = 4000))]
    #[schemars(extend("x-enabled-when" = { "quality": ["targetMb"] }))]
    target_mb: u32,
    /// 소리 유지
    ///
    /// 끄면 소리를 빼서 더 작아집니다.
    audio: bool,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            resolution: Resolution::Original,
            quality: Quality::Medium,
            target_mb: 25,
            audio: true,
        }
    }
}

/// 영상 확장자. 실제로 읽을 수 있는지는 Windows에 든 코덱이 정한다.
const VIDEO_EXTS: &[&str] = &["mp4", "mov", "m4v", "mkv", "avi", "wmv"];

/// 가장 낮게 허용하는 영상 비트레이트
const MIN_VIDEO_BPS: u32 = 150_000;

/// 짧은 변을 `short`로 줄인 크기 (짝수). 원본이 더 작으면 원본 크기.
fn scaled(w: u32, h: u32, short: Option<u32>) -> (u32, u32) {
    let even = |v: u32| (v & !1).max(2);
    match short {
        Some(s) if w.min(h) > s => {
            let r = s as f64 / w.min(h) as f64;
            // 가장 가까운 짝수 (H.264·NV12는 짝수 크기)
            let fit = |v: u32| ((v as f64 * r / 2.0).round() as u32 * 2).max(2);
            if w <= h {
                (even(s), fit(h))
            } else {
                (fit(w), even(s))
            }
        }
        _ => (even(w), even(h)),
    }
}

/// 영상 비트레이트 (bps). 화질 단계는 화소당 비트 수, 목표 용량은 길이로 나눈 값.
fn video_bitrate(
    quality: Quality,
    target_mb: u32,
    (w, h): (u32, u32),
    (fn_, fd): (u32, u32),
    duration: u64,
    audio: bool,
) -> Result<u32, ToolError> {
    let bpp = match quality {
        Quality::High => 0.1,
        Quality::Medium => 0.07,
        Quality::Low => 0.045,
        Quality::TargetMb => {
            let secs = duration as f64 / 1e7;
            // mp4 머리·색인 몫으로 4%를 남긴다
            let bits = target_mb.max(1) as f64 * 1024.0 * 1024.0 * 8.0 * 0.96;
            let audio_bps = if audio { mf::AUDIO_BPS as f64 } else { 0.0 };
            let bps = bits / secs - audio_bps;
            if bps < MIN_VIDEO_BPS as f64 {
                let min_mb =
                    ((MIN_VIDEO_BPS as f64 + audio_bps) * secs / 8.0 / 0.96 / 1024.0 / 1024.0)
                        .ceil();
                return Err(ToolError::InvalidOptions(format!(
                    "목표 용량이 영상 길이에 비해 너무 작습니다. {min_mb}MB 이상으로 정하거나 해상도를 낮춰 주세요."
                )));
            }
            return Ok(bps.min(u32::MAX as f64) as u32);
        }
    };
    let fps = fn_ as f64 / fd.max(1) as f64;
    Ok(((w as f64 * h as f64 * fps * bpp) as u32).max(MIN_VIDEO_BPS))
}

struct VideoCompress;

pub fn tool() -> Box<dyn Tool> {
    Box::new(VideoCompress)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for VideoCompress {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video.compress".into(),
            name: "영상 용량 줄이기".into(),
            category: "video".into(),
            description: "해상도와 화질을 낮춰 MP4(H.264) 영상으로 줄입니다.".into(),
            extensions: VIDEO_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(VideoOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: VideoOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("영상 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let probe = mf::probe(src)?;
        let (width, height) = scaled(probe.width, probe.height, o.resolution.short_side());
        let audio = o.audio && probe.has_audio;
        let video_bps = video_bitrate(
            o.quality,
            o.target_mb,
            (width, height),
            probe.fps,
            probe.duration,
            audio,
        )?;
        let mut plan = mf::Plan {
            format: mf::Format::Mp4,
            width,
            height,
            video_bps,
            audio,
        };
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let rel = PathBuf::from(format!("{stem}.mp4"));
        let dst = out_dir.join(&rel);
        // 인코더가 비트레이트를 넘길 수 있다: 목표 용량을 넘으면 넘은 만큼 낮춰 다시 (최대 3번)
        let limit = o.target_mb.max(1) as u64 * 1024 * 1024;
        for attempt in 1.. {
            mf::transcode(src, &dst, &probe, &plan, ctx)?;
            let len = std::fs::metadata(&dst)?.len();
            if o.quality != Quality::TargetMb || len <= limit {
                break;
            }
            let next = (plan.video_bps as f64 * limit as f64 / len as f64 * 0.95) as u32;
            if attempt == 3 || next < MIN_VIDEO_BPS {
                return Err(ToolError::Failed(format!(
                    "목표 용량({}MB)에 맞추지 못했습니다. 해상도를 낮추거나 '소리 유지'를 끄고 다시 해 보세요.",
                    o.target_mb
                )));
            }
            plan.video_bps = next;
        }
        Ok(ToolOutput::Files(vec![rel]))
    }
}
