//! 크롭 영역 계산 (옛 크롭 도구에서 옮김)

use crate::{Anchor, CropMode, EditOptions};

/// 자를 영역 (x, y, 너비, 높이). 직접 지정은 그 영역, 픽셀은 그 크기를 기준 위치에 맞춰 자른다 (원본보다 크면 원본 크기).
/// 크롭이 꺼져 있으면 전체.
pub(crate) fn crop_rect(w: u32, h: u32, o: &EditOptions) -> (u32, u32, u32, u32) {
    if !o.crop {
        return (0, 0, w, h);
    }
    let (cw, ch) = match o.crop_mode {
        // 직접 지정: 원본 밖으로 나간 부분은 잘라 맞춘다 (최소 1px)
        CropMode::Manual => {
            let x = o.crop_x.min(w - 1);
            let y = o.crop_y.min(h - 1);
            return (x, y, o.width.clamp(1, w - x), o.height.clamp(1, h - y));
        }
        CropMode::Pixels => (o.width.clamp(1, w), o.height.clamp(1, h)),
    };
    let (cw, ch) = (cw.max(1), ch.max(1));
    let (sx, sy) = (w - cw, h - ch);
    let (x, y) = match o.anchor {
        Anchor::Center => (sx / 2, sy / 2),
        Anchor::Top => (sx / 2, 0),
        Anchor::Bottom => (sx / 2, sy),
        Anchor::Left => (0, sy / 2),
        Anchor::Right => (sx, sy / 2),
        Anchor::TopLeft => (0, 0),
        Anchor::TopRight => (sx, 0),
        Anchor::BottomLeft => (0, sy),
        Anchor::BottomRight => (sx, sy),
    };
    (x, y, cw, ch)
}
