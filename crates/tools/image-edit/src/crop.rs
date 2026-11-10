//! 크롭 영역 계산 (옛 크롭 도구에서 옮김)

use crate::{Anchor, CropMode, EditOptions, Ratio};

impl Ratio {
    fn parts(self) -> (u32, u32) {
        match self {
            Ratio::R1x1 => (1, 1),
            Ratio::R4x3 => (4, 3),
            Ratio::R3x4 => (3, 4),
            Ratio::R16x9 => (16, 9),
            Ratio::R9x16 => (9, 16),
            Ratio::R3x2 => (3, 2),
            Ratio::R2x3 => (2, 3),
        }
    }
}

/// 자를 영역 (x, y, 너비, 높이). 비율은 원본 안에 들어가는 가장 큰 상자, 픽셀은 원본 크기로 자른다.
/// 크롭이 꺼져 있으면 전체.
pub(crate) fn crop_rect(w: u32, h: u32, o: &EditOptions) -> (u32, u32, u32, u32) {
    let (cw, ch) = match o.crop {
        CropMode::None => return (0, 0, w, h),
        CropMode::Pixels => (o.width.clamp(1, w), o.height.clamp(1, h)),
        CropMode::Ratio => {
            let (rw, rh) = o.ratio.parts();
            // 가로를 꽉 채웠을 때 세로가 넘치면 세로에 맞춘다
            let by_width = (w, (u64::from(w) * u64::from(rh) / u64::from(rw)) as u32);
            if by_width.1 <= h {
                by_width
            } else {
                ((u64::from(h) * u64::from(rw) / u64::from(rh)) as u32, h)
            }
        }
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
