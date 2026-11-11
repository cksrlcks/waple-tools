//! 워터마크를 이미지 위에 덮기: 9방향 한 곳 · 비스듬한 타일. 8비트·16비트 이미지 모두.

use image::{ImageBuffer, Pixel, Rgba, RgbaImage};

use crate::Position;

/// 타일 기울기 (화면에서 오른쪽 위로 올라가는 방향)
const TILE_ANGLE_DEG: f32 = -30.0;

/// 픽셀 채널 (u8·u16) ↔ 0~1
pub(crate) trait Channel: Copy {
    fn unit(self) -> f32;
    fn from_unit(v: f32) -> Self;
}

impl Channel for u8 {
    fn unit(self) -> f32 {
        f32::from(self) / 255.0
    }
    fn from_unit(v: f32) -> Self {
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

impl Channel for u16 {
    fn unit(self) -> f32 {
        f32::from(self) / 65535.0
    }
    fn from_unit(v: f32) -> Self {
        (v * 65535.0).round().clamp(0.0, 65535.0) as u16
    }
}

pub(crate) type Canvas<C> = ImageBuffer<Rgba<C>, Vec<C>>;

/// 직선 알파 픽셀 → 0~1 알파 곱한 값, 불투명도를 곱해서
fn premul(p: &Rgba<u8>, opacity: f32) -> [f32; 4] {
    let a = f32::from(p[3]) / 255.0 * opacity;
    [
        f32::from(p[0]) / 255.0 * a,
        f32::from(p[1]) / 255.0 * a,
        f32::from(p[2]) / 255.0 * a,
        a,
    ]
}

/// `src`(알파 곱한 값)를 `dst` 위에 덮는다 (source-over)
fn over<C: Channel>(dst: &mut Rgba<C>, [r, g, b, a]: [f32; 4]) {
    if a <= 0.0 {
        return;
    }
    let d = dst.0.map(C::unit);
    let k = d[3] * (1.0 - a);
    let out_a = a + k;
    let mix = |s: f32, c: f32| C::from_unit((s + c * k) / out_a);
    *dst = Rgba([
        mix(r, d[0]),
        mix(g, d[1]),
        mix(b, d[2]),
        C::from_unit(out_a),
    ]);
}

impl Position {
    /// 가로·세로 정렬: 0 시작, 1 가운데, 2 끝
    fn align(self) -> (u8, u8) {
        match self {
            Position::TopLeft => (0, 0),
            Position::Top => (1, 0),
            Position::TopRight => (2, 0),
            Position::Left => (0, 1),
            Position::Center | Position::Tile => (1, 1),
            Position::Right => (2, 1),
            Position::BottomLeft => (0, 2),
            Position::Bottom => (1, 2),
            Position::BottomRight => (2, 2),
        }
    }
}

/// 위치에 맞춰 덮는다: 타일이면 전체에 반복, 아니면 한 곳
pub(crate) fn stamp<C: Channel>(
    img: &mut Canvas<C>,
    mark: &RgbaImage,
    pos: Position,
    margin: u32,
    opacity: f32,
) where
    Rgba<C>: Pixel<Subpixel = C>,
{
    if pos == Position::Tile {
        tile(img, mark, opacity);
    } else {
        place(img, mark, pos, margin, opacity);
    }
}

/// 위치 한 곳에 덮는다. 시작·끝 정렬은 가장자리에서 `margin`만큼 띄운다.
fn place<C: Channel>(
    img: &mut Canvas<C>,
    mark: &RgbaImage,
    pos: Position,
    margin: u32,
    opacity: f32,
) where
    Rgba<C>: Pixel<Subpixel = C>,
{
    let start = |total: u32, size: u32, align: u8| match align {
        0 => margin,
        1 => total.saturating_sub(size) / 2,
        _ => total.saturating_sub(size.saturating_add(margin)),
    };
    let (ax, ay) = pos.align();
    let (x0, y0) = (
        start(img.width(), mark.width(), ax),
        start(img.height(), mark.height(), ay),
    );
    for (x, y, p) in mark.enumerate_pixels() {
        let (x, y) = (x0 + x, y0 + y);
        if x < img.width() && y < img.height() {
            over(img.get_pixel_mut(x, y), premul(p, opacity));
        }
    }
}

/// 이중 선형 보간 (알파 곱한 값). 바깥은 투명.
fn sample(mark: &RgbaImage, fx: f32, fy: f32, opacity: f32) -> [f32; 4] {
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let mut acc = [0.0; 4];
    for (dx, dy, wgt) in [
        (0, 0, (1.0 - tx) * (1.0 - ty)),
        (1, 0, tx * (1.0 - ty)),
        (0, 1, (1.0 - tx) * ty),
        (1, 1, tx * ty),
    ] {
        let (x, y) = (x0 as i64 + dx, y0 as i64 + dy);
        if x < 0 || y < 0 || x >= i64::from(mark.width()) || y >= i64::from(mark.height()) {
            continue;
        }
        let p = premul(mark.get_pixel(x as u32, y as u32), opacity);
        for (a, v) in acc.iter_mut().zip(p) {
            *a += v * wgt;
        }
    }
    acc
}

/// 이미지 가운데를 기준으로 비스듬한 격자에 반복해 덮는다. 줄마다 반 칸씩 엇갈린다.
fn tile<C: Channel>(img: &mut Canvas<C>, mark: &RgbaImage, opacity: f32)
where
    Rgba<C>: Pixel<Subpixel = C>,
{
    let (mw, mh) = (mark.width() as f32, mark.height() as f32);
    // 간격: 워터마크 긴 쪽의 절반 (로고는 1.5배 격자, 긴 텍스트는 줄 사이가 글자 높이 몇 배)
    let gap = mw.max(mh) / 2.0;
    let (su, sv) = (mw + gap, mh + gap);
    let (s, c) = TILE_ANGLE_DEG.to_radians().sin_cos();
    let (cx, cy) = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        // 워터마크 축으로 돌린 좌표 (가운데 칸의 워터마크가 이미지 가운데에 오게).
        // 칸 안에서 1px 띄워 감싸야 왼쪽·위 가장자리의 보간 픽셀이 앞 칸으로 넘어가 잘리지 않는다
        let u = dx * c + dy * s + mw / 2.0 + 1.0;
        let v = -dx * s + dy * c + mh / 2.0 + 1.0;
        let row = (v / sv).floor() as i64;
        let shift = if row.rem_euclid(2) == 1 {
            su / 2.0
        } else {
            0.0
        };
        let (uu, vv) = ((u + shift).rem_euclid(su), v.rem_euclid(sv));
        if uu < mw + 2.0 && vv < mh + 2.0 {
            over(p, sample(mark, uu - 1.5, vv - 1.5, opacity));
        }
    }
}
