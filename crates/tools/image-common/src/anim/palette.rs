//! 모든 프레임이 함께 쓰는 팔레트 하나. 마지막 칸은 투명(투명 화소 · 앞 프레임 유지)용.

use std::collections::{HashMap, HashSet};

use color_quant::NeuQuant;
use image::Rgba;

use super::GifFrame as Frame;

/// NeuQuant 학습에 넘길 최대 화소 수 (긴 GIF 전체를 복사하지 않게 고르게 뽑는다)
const SAMPLE: usize = 1_000_000;

enum Mapper {
    /// 색이 적으면 원래 색 그대로 (NeuQuant는 색을 조금씩 바꾼다)
    Exact(HashMap<[u8; 3], u8>),
    Quant(NeuQuant),
}

pub(crate) struct Palette {
    /// 전역 팔레트 RGB (투명 칸 포함)
    pub rgb: Vec<u8>,
    pub transparent: u8,
    mapper: Mapper,
}

impl Palette {
    pub fn index(&self, p: &Rgba<u8>) -> u8 {
        if p[3] < 128 {
            return self.transparent;
        }
        match &self.mapper {
            Mapper::Exact(m) => m[&[p[0], p[1], p[2]]],
            Mapper::Quant(nq) => nq.index_of(&[p[0], p[1], p[2], 255]) as u8,
        }
    }
}

fn opaque(frames: &[Frame]) -> impl Iterator<Item = &Rgba<u8>> {
    frames
        .iter()
        .flat_map(|f| f.rgba.pixels())
        .filter(|p| p[3] >= 128)
}

/// 서로 다른 색이 `max`개 이하면 그 목록
fn exact_colors(frames: &[Frame], max: usize) -> Option<Vec<[u8; 3]>> {
    let mut seen = HashSet::new();
    let mut list = Vec::new();
    for p in opaque(frames) {
        let c = [p[0], p[1], p[2]];
        if seen.insert(c) {
            if list.len() == max {
                return None;
            }
            list.push(c);
        }
    }
    Some(list)
}

/// `colors`(2~256)는 실제 색 수. 투명 칸을 더해 GIF 팔레트(최대 256칸)에 넣으려고 256은 255로
pub(crate) fn build(frames: &[Frame], colors: usize) -> Palette {
    let max = colors.min(255);
    let (mut rgb, mapper) = match exact_colors(frames, max) {
        Some(list) => (
            list.concat(),
            Mapper::Exact(
                list.iter()
                    .enumerate()
                    .map(|(i, c)| (*c, i as u8))
                    .collect(),
            ),
        ),
        None => {
            let total: usize = frames.iter().map(|f| f.rgba.len() / 4).sum();
            let sample: Vec<u8> = opaque(frames)
                .step_by((total / SAMPLE).max(1))
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect();
            let nq = NeuQuant::new(10, max, &sample);
            (nq.color_map_rgb(), Mapper::Quant(nq))
        }
    };
    let transparent = (rgb.len() / 3) as u8;
    rgb.extend([0, 0, 0]);
    Palette {
        rgb,
        transparent,
        mapper,
    }
}
