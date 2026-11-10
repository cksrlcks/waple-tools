//! 이미지 도구 공용: 읽기(EXIF 방향 반영) · 포맷별 인코딩 · 메타데이터 · 결과 쓰기

mod display;
mod encode;
mod format;
mod load;
mod meta;
mod output;
#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
#[cfg(test)]
mod tests;
mod tool_util;

pub use display::{DisplayImage, display_image};
pub use encode::{Quality, encode, flatten, optimize_png, parse_color};
pub use format::{Fmt, INPUT_EXTS};
pub use load::{Loaded, load};
pub use meta::{Meta, MetaMode, apply_meta, exif_reset_orientation, exif_without_gps, with_meta};

/// 결과 이미지 화소 수 상한 (1억 화소). 여백·캔버스로 수 GB를 할당하지 않게 한다.
pub const MAX_PIXELS: u64 = 100_000_000;

/// 결과 크기가 상한을 넘으면 실패
pub fn ensure_pixels(w: u32, h: u32) -> Result<(), localtools_core::ToolError> {
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(localtools_core::ToolError::Failed(
            "결과 이미지가 너무 큽니다 (1억 화소 초과).".into(),
        ));
    }
    Ok(())
}
pub use output::write_output;
pub use tool_util::{parse_options, run_each, schema_of};
