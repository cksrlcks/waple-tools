//! 움직이는 GIF 저장: 모든 프레임 공용 팔레트 + 바뀐 부분만 다시 그리기 (GIF 최적화·영상 → GIF)

mod encode;
mod palette;

pub use gif::Repeat;
use image::RgbaImage;
use waple_tools_core::{JobContext, ToolError};

/// 합성된 전체 크기 프레임
pub struct GifFrame {
    pub rgba: RgbaImage,
    pub delay_ms: u32,
}

/// 같은 크기(`w`×`h`) 프레임들을 색 `colors`개(2~256)로 줄여 GIF로 만든다. 진행률(0~1)은 `on_progress`로 알린다
pub fn encode_gif(
    frames: &[GifFrame],
    w: u32,
    h: u32,
    colors: usize,
    repeat: Repeat,
    ctx: &JobContext,
    on_progress: &dyn Fn(f32),
) -> Result<Vec<u8>, ToolError> {
    let pal = palette::build(frames, colors.clamp(2, 256));
    encode::write(frames, w, h, &pal, repeat, ctx, on_progress)
}
