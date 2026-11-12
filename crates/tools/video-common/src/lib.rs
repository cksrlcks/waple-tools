//! 영상 도구 공용: Windows Media Foundation으로 정보 읽기 · 다시 인코딩 · RGBA 프레임 읽기 · RGBA → mp4

mod frames;
mod mf;
#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
#[cfg(test)]
mod tests;
mod writer;

pub use frames::{Frame, FrameReader};
pub use mf::{AUDIO_BPS, Format, Plan, Probe, Session, probe, transcode};
pub use writer::{MAX_ENCODE_SIDE, MIN_ENCODE_SIDE, Mp4Writer};

/// 영상 도구가 받는 확장자. 실제로 읽을 수 있는지는 Windows에 든 코덱이 정한다.
pub const VIDEO_EXTS: &[&str] = &["mp4", "mov", "m4v", "mkv", "avi", "wmv", "webm"];

/// 크기를 짝수로 내린다 (H.264·NV12는 짝수 크기, 최소 2)
pub fn even(v: u32) -> u32 {
    (v & !1).max(2)
}

/// 가로를 `width` 이하로 줄인 크기 (비율 유지, 짝수). 원본이 더 작으면 원본 크기 (확대 안 함)
pub fn fit_width(w: u32, h: u32, width: Option<u32>) -> (u32, u32) {
    match width {
        Some(tw) if w > tw => {
            let nh = ((h as f64 * tw as f64 / w as f64 / 2.0).round() as u32 * 2).max(2);
            (even(tw), nh)
        }
        _ => (even(w), even(h)),
    }
}
