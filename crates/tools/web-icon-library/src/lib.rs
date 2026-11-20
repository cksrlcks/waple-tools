//! 웹 아이콘 라이브러리 추가 도구. 무료 Iconify 세트 목록(스냅샷), SVG 본문 정리, 받아 둔 세트 읽기.
//! 디스크에는 쓰지 않고(읽기만), 네트워크도 쓰지 않는다. 받기는 앱의 공용 리소스 받기가 맡는다.

pub mod collections;
pub mod sanitize;
pub mod set;

#[cfg(test)]
mod tests;
