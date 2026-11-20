//! 아이콘 라이브러리: npm(@iconify-json/*)에서 세트를 받아 SVG로 보관한다.

// 저장소·명령(이후 Task)이 쓰기 전까지 미사용 경고를 막는다.
#[allow(dead_code)]
pub mod npm;
#[allow(dead_code)]
pub mod sanitize;
#[allow(dead_code)]
pub mod set;
#[allow(dead_code)]
pub mod unpack;

#[cfg(test)]
mod tests;
