//! 역할별 색: node-vibrant 기본 생성기(= Android Palette 방식)를 옮겼다

use crate::color::{Rgb, from_hsl, to_hsl};
use crate::mmcq::Swatch;

/// 화면·CSS·JSON에 나오는 순서
pub const ROLES: [&str; 6] = [
    "Vibrant",
    "DarkVibrant",
    "LightVibrant",
    "Muted",
    "DarkMuted",
    "LightMuted",
];
const VIBRANT: usize = 0;
const DARK_VIBRANT: usize = 1;
const LIGHT_VIBRANT: usize = 2;
const MUTED: usize = 3;
const DARK_MUTED: usize = 4;
const LIGHT_MUTED: usize = 5;

const TARGET_DARK_LUMA: f64 = 0.26;
const MAX_DARK_LUMA: f64 = 0.45;
const MIN_LIGHT_LUMA: f64 = 0.55;
const TARGET_LIGHT_LUMA: f64 = 0.74;
const MIN_NORMAL_LUMA: f64 = 0.3;
const TARGET_NORMAL_LUMA: f64 = 0.5;
const MAX_NORMAL_LUMA: f64 = 0.7;
const TARGET_MUTES_SATURATION: f64 = 0.3;
const MAX_MUTES_SATURATION: f64 = 0.4;
const TARGET_VIBRANT_SATURATION: f64 = 1.0;
const MIN_VIBRANT_SATURATION: f64 = 0.35;
// 가중치는 node-vibrant v3 DefaultGenerator 기본값 (원조 Vibrant.js는 3 / 6 / 1)
const WEIGHT_SATURATION: f64 = 3.0;
const WEIGHT_LUMA: f64 = 6.5;
const WEIGHT_POPULATION: f64 = 0.5;

/// (최소, 목표, 최대)
type Range = (f64, f64, f64);
const DARK: Range = (0.0, TARGET_DARK_LUMA, MAX_DARK_LUMA);
const NORMAL: Range = (MIN_NORMAL_LUMA, TARGET_NORMAL_LUMA, MAX_NORMAL_LUMA);
const LIGHT: Range = (MIN_LIGHT_LUMA, TARGET_LIGHT_LUMA, 1.0);
const VIVID: Range = (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0);
const MUTE: Range = (0.0, TARGET_MUTES_SATURATION, MAX_MUTES_SATURATION);

/// node-vibrant가 고르는 순서 (앞에서 고른 색은 뒤에서 다시 고르지 않는다)
const PICK_ORDER: [(usize, Range, Range); 6] = [
    (VIBRANT, NORMAL, VIVID),
    (LIGHT_VIBRANT, LIGHT, VIVID),
    (DARK_VIBRANT, DARK, VIVID),
    (MUTED, NORMAL, MUTE),
    (LIGHT_MUTED, LIGHT, MUTE),
    (DARK_MUTED, DARK, MUTE),
];

#[derive(Clone, Copy, Debug)]
pub struct Pick {
    pub rgb: Rgb,
    pub population: u32,
    /// 이미지에 없어 다른 역할의 색에서 만든 색
    pub derived: bool,
}

fn score(s: &Swatch, sat: f64, luma: f64, t_sat: f64, t_luma: f64, max_pop: f64) -> f64 {
    let inv = |v: f64, t: f64| 1.0 - (v - t).abs();
    (inv(sat, t_sat) * WEIGHT_SATURATION
        + inv(luma, t_luma) * WEIGHT_LUMA
        + f64::from(s.population) / max_pop * WEIGHT_POPULATION)
        / (WEIGHT_SATURATION + WEIGHT_LUMA + WEIGHT_POPULATION)
}

/// 역할마다 색 하나. 맞는 색이 없고 만들 수도 없으면 None
pub fn roles(swatches: &[Swatch]) -> [Option<Pick>; 6] {
    let max_pop = f64::from(swatches.iter().map(|s| s.population).max().unwrap_or(1));
    let mut chosen: [Option<usize>; 6] = [None; 6];
    for (role, luma, sat) in PICK_ORDER {
        let mut best: Option<(usize, f64)> = None;
        for (i, s) in swatches.iter().enumerate() {
            let [_, s_, l] = to_hsl(s.rgb);
            let fits = (sat.0..=sat.2).contains(&s_) && (luma.0..=luma.2).contains(&l);
            if !fits || chosen.contains(&Some(i)) {
                continue;
            }
            let v = score(s, s_, l, sat.1, luma.1, max_pop);
            if best.is_none_or(|(_, b)| v > b) {
                best = Some((i, v));
            }
        }
        chosen[role] = best.map(|(i, _)| i);
    }
    let mut p = chosen.map(|c| {
        c.map(|i| Pick {
            rgb: swatches[i].rgb,
            population: swatches[i].population,
            derived: false,
        })
    });
    fill_empty(&mut p);
    p
}

fn derive(src: Pick, f: impl FnOnce(&mut [f64; 3])) -> Option<Pick> {
    let mut hsl = to_hsl(src.rgb);
    f(&mut hsl);
    Some(Pick {
        rgb: from_hsl(hsl),
        population: 0,
        derived: true,
    })
}

fn with_luma(src: Pick, l: f64) -> Option<Pick> {
    derive(src, |hsl| hsl[2] = l)
}

fn with_saturation(src: Pick, s: f64) -> Option<Pick> {
    derive(src, |hsl| hsl[1] = s)
}

/// node-vibrant의 _generateEmptySwatches. 원본의 오타로 보이는 두 곳은 의도대로 고쳤다:
/// LightMuted로 DarkVibrant를 덮어쓰던 것 → LightVibrant를 만든다,
/// Muted 계열을 만들 때 명도를 TARGET_MUTES_SATURATION으로 바꾸던 것 → 채도를 바꾼다.
fn fill_empty(p: &mut [Option<Pick>; 6]) {
    if p[VIBRANT].is_none() && p[DARK_VIBRANT].is_none() && p[LIGHT_VIBRANT].is_none() {
        if let Some(dm) = p[DARK_MUTED] {
            p[DARK_VIBRANT] = with_luma(dm, TARGET_DARK_LUMA);
        }
        if let Some(lm) = p[LIGHT_MUTED] {
            p[LIGHT_VIBRANT] = with_luma(lm, TARGET_LIGHT_LUMA);
        }
    }
    if p[VIBRANT].is_none()
        && let Some(src) = p[DARK_VIBRANT].or(p[LIGHT_VIBRANT])
    {
        p[VIBRANT] = with_luma(src, TARGET_NORMAL_LUMA);
    }
    if let Some(v) = p[VIBRANT] {
        if p[DARK_VIBRANT].is_none() {
            p[DARK_VIBRANT] = with_luma(v, TARGET_DARK_LUMA);
        }
        if p[LIGHT_VIBRANT].is_none() {
            p[LIGHT_VIBRANT] = with_luma(v, TARGET_LIGHT_LUMA);
        }
        if p[MUTED].is_none() {
            p[MUTED] = with_saturation(v, TARGET_MUTES_SATURATION);
        }
    }
    if p[LIGHT_MUTED].is_none()
        && let Some(lv) = p[LIGHT_VIBRANT]
    {
        p[LIGHT_MUTED] = with_saturation(lv, TARGET_MUTES_SATURATION);
    }
    if p[DARK_MUTED].is_none()
        && let Some(dv) = p[DARK_VIBRANT]
    {
        p[DARK_MUTED] = with_saturation(dv, TARGET_MUTES_SATURATION);
    }
}
