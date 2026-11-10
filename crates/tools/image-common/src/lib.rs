//! 이미지 도구 공용: 읽기(EXIF 방향 반영) · 포맷별 인코딩 · 메타데이터 · 결과 쓰기

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

pub use encode::{Quality, encode, flatten, optimize_png, parse_color};
pub use format::{Fmt, INPUT_EXTS};
pub use load::{Loaded, load};
pub use meta::{Meta, MetaMode, exif_reset_orientation, exif_without_gps, with_meta};
pub use output::write_output;
pub use tool_util::{parse_options, run_each, schema_of};
