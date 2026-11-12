//! 영상 썸네일 추출 (추가 설치, Generator): 균등하게 N장 / N초 간격 / 지정 시간 → `<이름>/<이름>_00m12s.jpg` …

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use image::DynamicImage;
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{Fmt, Quality, encode, parse_options, schema_of};
use waple_tools_video_common as mf;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 균등하게 N장
    #[default]
    Count,
    /// N초 간격
    Interval,
    /// 지정 시간
    Times,
}

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

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct ThumbOptions {
    /// 방식
    mode: Mode,
    /// 장 수
    ///
    /// 영상 전체에 고르게 나눠 뽑습니다 (맨 처음·맨 끝 화면은 빼고 구간 가운데).
    #[schemars(range(min = 1, max = 100))]
    #[schemars(extend("x-enabled-when" = { "mode": ["count"] }))]
    count: u32,
    /// 간격(초)
    ///
    /// 0초부터 이 간격마다 뽑습니다. 최대 100장입니다.
    #[schemars(range(min = 1))]
    #[schemars(extend("x-enabled-when" = { "mode": ["interval"] }))]
    interval: u32,
    /// 시간 목록
    ///
    /// 쉼표나 공백으로 구분합니다. 초(75), 분:초(1:15), 시:분:초(1:02:03) 형식을 씁니다. 최대 100개입니다.
    #[schemars(extend("x-enabled-when" = { "mode": ["times"] }, "x-wide" = true))]
    times: String,
    /// 포맷
    format: Format,
    /// 너비
    ///
    /// 원본보다 작을 때만 줄이고 키우지는 않습니다.
    width: Width,
}

impl Default for ThumbOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Count,
            count: 10,
            interval: 10,
            times: String::new(),
            format: Format::Jpg,
            width: Width::Original,
        }
    }
}

const MAX_SHOTS: usize = 100;
const JPEG_QUALITY: u8 = 90;

/// `12`, `1:30`, `1:02:03.5` → 초
fn parse_time(s: &str) -> Option<f64> {
    let mut secs = 0.0;
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        let last = i == parts.len() - 1;
        let v: f64 = if last {
            p.parse().ok()?
        } else {
            f64::from(p.parse::<u32>().ok()?)
        };
        // 앞자리가 있으면 분·초는 60 미만
        if !v.is_finite() || v < 0.0 || (i > 0 && v >= 60.0) {
            return None;
        }
        secs = secs * 60.0 + v;
    }
    Some(secs)
}

/// 뽑을 시각들(초, 오름차순, 중복 없음)
/// `total`(초)이 `None`이면 영상 길이를 모르는 것: 지정 시간만 된다
fn shot_times(o: &ThumbOptions, total: Option<f64>) -> Result<Vec<f64>, ToolError> {
    let need_total = || {
        total.ok_or_else(|| {
            ToolError::InvalidOptions(
                "영상 길이를 알 수 없어 고르게·간격으로 뽑을 수 없습니다. 방식을 '지정 시간'으로 바꿔 주세요.".into(),
            )
        })
    };
    let too_many = |n: usize| {
        ToolError::InvalidOptions(format!(
            "{n}장이 됩니다. 한 번에 최대 {MAX_SHOTS}장까지 뽑을 수 있습니다."
        ))
    };
    let list: Vec<f64> = match o.mode {
        Mode::Count => {
            let total = need_total()?;
            let n = o.count.clamp(1, MAX_SHOTS as u32);
            (0..n)
                .map(|i| total * f64::from(2 * i + 1) / f64::from(2 * n))
                .collect()
        }
        Mode::Interval => {
            let total = need_total()?;
            let iv = f64::from(o.interval.max(1));
            let n = (total / iv).ceil() as usize;
            if n > MAX_SHOTS {
                return Err(too_many(n));
            }
            (0..n).map(|i| i as f64 * iv).collect()
        }
        Mode::Times => {
            let mut list = vec![];
            for t in o
                .times
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|t| !t.is_empty())
            {
                let s = parse_time(t).ok_or_else(|| {
                    ToolError::InvalidOptions(format!(
                        "시간 형식이 올바르지 않습니다: {t} (예: 75, 1:15, 1:02:03)"
                    ))
                })?;
                if let Some(total) = total.filter(|&total| s >= total) {
                    return Err(ToolError::InvalidOptions(format!(
                        "{t}는 영상 길이({})를 넘습니다.",
                        label(total, false)
                    )));
                }
                list.push(s);
            }
            if list.is_empty() {
                return Err(ToolError::InvalidOptions(
                    "뽑을 시간을 하나 이상 입력해 주세요.".into(),
                ));
            }
            if list.len() > MAX_SHOTS {
                return Err(too_many(list.len()));
            }
            list
        }
    };
    // 이름표는 밀리초 단위라 밀리초로 반올림해 겹치는 시각을 하나로 (같은 이름 = 결과 세트 저장 실패)
    let mut list: Vec<f64> = list
        .into_iter()
        .map(|t| (t * 1000.0).round() / 1000.0)
        .collect();
    list.sort_by(f64::total_cmp);
    list.dedup();
    Ok(list)
}

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

/// 시각마다 이름표. 초 단위로 겹치면 모두 밀리초까지
fn labels(times: &[f64]) -> Vec<String> {
    let plain: Vec<String> = times.iter().map(|&t| label(t, false)).collect();
    if plain.windows(2).any(|w| w[0] == w[1]) {
        times.iter().map(|&t| label(t, true)).collect()
    } else {
        plain
    }
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
            description: "영상의 장면을 JPG·PNG 이미지로 저장합니다.".into(),
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
        let o: ThumbOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("영상 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let probe = mf::probe(src)?;
        let times = shot_times(&o, probe.secs())?;
        let (sw, sh) = probe.shown_size();
        let (w, h) = mf::fit_width(sw, sh, o.width.px());
        let mut reader = mf::FrameReader::open(src, &probe, w, h)?;
        let (fmt, ext) = match o.format {
            Format::Jpg => (Fmt::Jpeg, "jpg"),
            Format::Png => (Fmt::Png, "png"),
        };
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let top = PathBuf::from(stem.as_ref());
        std::fs::create_dir_all(out_dir.join(&top))?;
        let mut names = Vec::with_capacity(times.len());
        for (i, (&t, tag)) in times.iter().zip(labels(&times)).enumerate() {
            ctx.check_cancel()?;
            let at = (t * 1e7) as i64;
            reader.seek(at as u64)?;
            // 키프레임부터 나온다: 그 시각에 화면에 있던 프레임 = 시각이 그 이하인 마지막 프레임 (없으면 첫 프레임, 끝이면 마지막)
            let mut shot = None;
            while let Some(f) = reader.next_frame()? {
                if f.time > at {
                    shot = shot.or(Some(f));
                    break;
                }
                shot = Some(f);
                ctx.check_cancel()?;
            }
            let f = shot.ok_or_else(|| {
                ToolError::Failed(format!("{}의 화면을 읽지 못했습니다.", label(t, false)))
            })?;
            let img =
                DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(reader.image(&f)?).to_rgb8());
            let data = encode(
                &img,
                fmt,
                Quality {
                    quality: JPEG_QUALITY,
                    lossless: false,
                },
            )?;
            let rel = top.join(format!("{stem}_{tag}.{ext}"));
            std::fs::write(out_dir.join(&rel), data)?;
            names.push(rel);
            ctx.progress((i + 1) as f32 / times.len() as f32);
        }
        Ok(ToolOutput::Files(names))
    }
}
