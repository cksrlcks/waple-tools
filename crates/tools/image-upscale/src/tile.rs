//! 타일 추론: 모델 입력이 128×128 고정이라 이미지를 겹치게 잘라 넣고, 겹친 곳의 가운데에서 이어 붙인다.
//! 이음매마다 양쪽에 PAD 이상 문맥이 있어 타일 경계가 보이지 않는다.

use image::imageops::{FilterType, resize};
use image::{Rgb, RgbImage};
use waple_tools_core::ToolError;

/// 모델 입력 한 변
pub const TILE: u32 = 128;
/// 모델 배율
pub const MODEL_SCALE: u32 = 4;
/// 이음매 양쪽에 남길 최소 문맥(입력 픽셀)
pub const PAD: u32 = 10;
/// 이웃 타일 시작점 사이 최대 거리
const STEP: u32 = TILE - 2 * PAD;

/// 한 축의 타일들: (입력 창 시작, 결과로 쓸 구간 시작, 끝). 창은 `[시작, 시작 + TILE)`.
/// `len`이 TILE보다 짧으면 창이 이미지를 넘는다 (넘는 곳은 가장자리 픽셀을 늘려 채운다).
pub fn spans(len: u32) -> Vec<(u32, u32, u32)> {
    let padded = len.max(TILE);
    let room = padded - TILE;
    let n = room.div_ceil(STEP) + 1;
    // 창을 고르게 펼친다: 이웃 사이 거리 ≤ STEP → 겹침 ≥ 2·PAD
    let starts: Vec<u32> = (0..n)
        .map(|i| {
            if n == 1 {
                0
            } else {
                (u64::from(i) * u64::from(room) / u64::from(n - 1)) as u32
            }
        })
        .collect();
    let mut out = Vec::with_capacity(n as usize);
    let mut from = 0;
    for (i, &s) in starts.iter().enumerate() {
        let to = starts
            .get(i + 1)
            .map_or(padded, |&next| (s + TILE + next) / 2)
            .min(len);
        out.push((s, from, to));
        from = to;
    }
    out
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// RGB 이미지를 `scale`(2 또는 4)배로 키운다.
/// `infer`: NCHW 0~1 입력 `3×TILE×TILE` → 결과 `3×(4·TILE)×(4·TILE)`.
/// 2배는 타일 결과를 고품질(Lanczos3)로 반 줄여 붙인다: 이음매 문맥이 필터 폭보다 넓어 전체를 한 번에 줄인 것과 같다.
/// `on_tile(끝난 수, 전체)`가 오류를 내면(취소) 멈춘다.
pub fn upscale_rgb(
    img: &RgbImage,
    scale: u32,
    mut infer: impl FnMut(&[f32]) -> Result<Vec<f32>, ToolError>,
    mut on_tile: impl FnMut(usize, usize) -> Result<(), ToolError>,
) -> Result<RgbImage, ToolError> {
    let (w, h) = img.dimensions();
    let (xs, ys) = (spans(w), spans(h));
    let total = xs.len() * ys.len();
    let big = TILE * MODEL_SCALE;
    let plane = (TILE * TILE) as usize;
    let mut out = RgbImage::new(w * scale, h * scale);
    let mut input = vec![0f32; 3 * plane];
    let mut done = 0;
    for &(sy, y0, y1) in &ys {
        for &(sx, x0, x1) in &xs {
            for ty in 0..TILE {
                for tx in 0..TILE {
                    let p = img.get_pixel((sx + tx).min(w - 1), (sy + ty).min(h - 1));
                    let i = (ty * TILE + tx) as usize;
                    for c in 0..3 {
                        input[c * plane + i] = f32::from(p[c]) / 255.0;
                    }
                }
            }
            let o = infer(&input)?;
            let n = (big * big) as usize;
            if o.len() != 3 * n {
                return Err(ToolError::Failed(
                    "AI 모델 결과 크기가 올바르지 않습니다.".into(),
                ));
            }
            let mut tile = RgbImage::from_fn(big, big, |x, y| {
                let i = (y * big + x) as usize;
                Rgb([to_u8(o[i]), to_u8(o[n + i]), to_u8(o[2 * n + i])])
            });
            if scale != MODEL_SCALE {
                let side = TILE * scale;
                tile = resize(&tile, side, side, FilterType::Lanczos3);
            }
            for y in y0 * scale..y1 * scale {
                for x in x0 * scale..x1 * scale {
                    out.put_pixel(x, y, *tile.get_pixel(x - sx * scale, y - sy * scale));
                }
            }
            done += 1;
            on_tile(done, total)?;
        }
    }
    Ok(out)
}
