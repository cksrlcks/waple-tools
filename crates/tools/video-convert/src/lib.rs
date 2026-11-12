//! 영상 포맷 변환 (추가 설치, PerFile): Windows가 읽을 수 있는 영상 → MP4(H.264+AAC) · WMV(WMV9+WMA) · 소리만 M4A(AAC) · MP3. 해상도는 원본 그대로.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Format {
    /// MP4 (H.264)
    #[default]
    Mp4,
    /// WMV
    Wmv,
    /// M4A (소리만, AAC)
    M4a,
    /// MP3 (소리만)
    Mp3,
}

impl Format {
    fn mf(self) -> mf::Format {
        match self {
            Self::Mp4 => mf::Format::Mp4,
            Self::Wmv => mf::Format::Wmv,
            Self::M4a => mf::Format::M4a,
            Self::Mp3 => mf::Format::Mp3,
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
struct ConvertOptions {
    /// 출력 형식
    ///
    /// 해상도는 원본 그대로이고 화질은 '영상 용량 줄이기'의 '높음'과 같습니다. M4A·MP3는 영상을 빼고 소리만 저장합니다.
    format: Format,
    /// 소리 유지
    ///
    /// 끄면 소리를 빼고 영상만 저장합니다.
    #[schemars(extend("x-enabled-when" = { "format": ["mp4", "wmv"] }))]
    audio: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            format: Format::Mp4,
            audio: true,
        }
    }
}

/// 화질 '높음'의 화소당 비트 수 (video.compress와 같은 값)
const HIGH_BPP: f64 = 0.1;
const MIN_VIDEO_BPS: u32 = 150_000;

struct VideoConvert;

pub fn tool() -> Box<dyn Tool> {
    Box::new(VideoConvert)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for VideoConvert {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video.convert".into(),
            name: "영상 포맷 변환".into(),
            category: "video".into(),
            description: "영상을 MP4·WMV로 바꾸거나 소리만 M4A·MP3로 뽑습니다.".into(),
            extensions: mf::VIDEO_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(ConvertOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: ConvertOptions = serde_json::from_value(if opts.is_null() {
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
        let format = o.format.mf();
        let audio = if format.has_video() {
            o.audio && probe.has_audio
        } else if probe.has_audio {
            true
        } else {
            return Err(ToolError::Failed(
                "소리가 없는 영상이라 M4A·MP3로 뽑을 수 없습니다.".into(),
            ));
        };
        let (width, height) = mf::fit_width(probe.width, probe.height, None);
        let fps = probe.fps.0 as f64 / probe.fps.1.max(1) as f64;
        let video_bps = ((width as f64 * height as f64 * fps * HIGH_BPP) as u32).max(MIN_VIDEO_BPS);
        let plan = mf::Plan {
            format,
            width,
            height,
            video_bps,
            audio,
        };
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let rel = PathBuf::from(format!("{stem}.{}", format.ext()));
        mf::transcode(src, &out_dir.join(&rel), &probe, &plan, ctx)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
