//! MMCQ (수정 median cut 양자화: Leptonica → color-thief → node-vibrant). 같은 입력이면 늘 같은 결과.
//! node-vibrant와 달리 칸 가운데가 아니라 실제 픽셀 평균을 색으로 써서 단색이 정확히 나온다.

use crate::color::Rgb;

/// 채널마다 5비트(32칸)로 묶는다
const SIDE: usize = 32;
/// 1단계(픽셀 수 기준)로 나눌 상자 비율. 나머지는 픽셀 수×부피 기준으로 나눠 작은 강조색도 살린다
const FRACT_BY_POPULATION: f64 = 0.75;

#[derive(Clone, Copy, Debug)]
pub struct Swatch {
    pub rgb: Rgb,
    pub population: u32,
}

struct Hist {
    count: Vec<u32>,
    sum: Vec<[u64; 3]>,
}

fn idx(c: [usize; 3]) -> usize {
    (c[0] * SIDE + c[1]) * SIDE + c[2]
}

#[derive(Clone, Copy)]
struct VBox {
    lo: [usize; 3],
    hi: [usize; 3],
    count: u32,
}

impl VBox {
    fn volume(&self) -> u64 {
        (0..3)
            .map(|a| (self.hi[a] - self.lo[a] + 1) as u64)
            .product()
    }
}

fn bins(lo: [usize; 3], hi: [usize; 3]) -> impl Iterator<Item = [usize; 3]> {
    (lo[0]..=hi[0]).flat_map(move |r| {
        (lo[1]..=hi[1]).flat_map(move |g| (lo[2]..=hi[2]).map(move |b| [r, g, b]))
    })
}

/// 빈 칸을 걷어 낸 꼭 맞는 상자. 비었으면 None
fn fit(h: &Hist, lo: [usize; 3], hi: [usize; 3]) -> Option<VBox> {
    let (mut nlo, mut nhi, mut count) = (hi, lo, 0);
    for c in bins(lo, hi) {
        let n = h.count[idx(c)];
        if n == 0 {
            continue;
        }
        count += n;
        for a in 0..3 {
            nlo[a] = nlo[a].min(c[a]);
            nhi[a] = nhi[a].max(c[a]);
        }
    }
    (count > 0).then_some(VBox {
        lo: nlo,
        hi: nhi,
        count,
    })
}

/// 가장 긴 축을 픽셀 수 중앙값 근처에서 자른다.
/// color-thief의 doCut처럼 긴 쪽으로 절반만큼 밀어 자르므로 멀리 떨어진 색이 따로 떨어진다.
fn split(h: &Hist, b: &VBox) -> Option<(VBox, VBox)> {
    // 길이가 같으면 r → g → b 순 (color-thief와 같다)
    let axis = (0..3).rev().max_by_key(|&a| b.hi[a] - b.lo[a])?;
    let (lo, hi) = (b.lo[axis], b.hi[axis]);
    if lo == hi {
        return None;
    }
    let mut partial = vec![0u64; hi - lo + 1];
    for c in bins(b.lo, b.hi) {
        partial[c[axis] - lo] += u64::from(h.count[idx(c)]);
    }
    for k in 1..partial.len() {
        partial[k] += partial[k - 1];
    }
    let total = u64::from(b.count);
    let i = (lo + partial.iter().position(|&p| 2 * p > total)?) as isize;
    let (lo_i, hi_i) = (lo as isize, hi as isize);
    let (left, right) = (i - lo_i, hi_i - i);
    // 상자를 꼭 맞게 줄였으므로 양 끝 칸은 비어 있지 않다 → 두 상자 모두 비지 않는다
    let d = if left <= right {
        (i + right / 2).min(hi_i - 1)
    } else {
        (i - 1 - (left + 1) / 2).max(lo_i)
    } as usize;
    let (mut hi1, mut lo2) = (b.hi, b.lo);
    hi1[axis] = d;
    lo2[axis] = d + 1;
    Some((fit(h, b.lo, hi1)?, fit(h, lo2, b.hi)?))
}

fn split_until(
    h: &Hist,
    boxes: &mut Vec<VBox>,
    done: &mut Vec<VBox>,
    target: usize,
    key: impl Fn(&VBox) -> u64,
) {
    while boxes.len() + done.len() < target {
        // 값이 같으면 먼저 들어온 상자 (결정적)
        let Some(i) = (0..boxes.len()).rev().max_by_key(|&i| key(&boxes[i])) else {
            return;
        };
        let b = boxes.remove(i);
        match split(h, &b) {
            Some((x, y)) => boxes.extend([x, y]),
            None => done.push(b),
        }
    }
}

/// 많아야 `max`개 색. 서로 다른 색이 적으면 그만큼만 나온다
pub fn quantize(pixels: &[Rgb], max: usize) -> Vec<Swatch> {
    let mut h = Hist {
        count: vec![0; SIDE.pow(3)],
        sum: vec![[0; 3]; SIDE.pow(3)],
    };
    for p in pixels {
        let i = idx(p.map(|v| usize::from(v >> 3)));
        h.count[i] += 1;
        for (sum, &v) in h.sum[i].iter_mut().zip(p) {
            *sum += u64::from(v);
        }
    }
    let Some(first) = fit(&h, [0; 3], [SIDE - 1; 3]) else {
        return vec![];
    };
    let (mut boxes, mut done) = (vec![first], vec![]);
    let by_population = (FRACT_BY_POPULATION * max as f64).ceil() as usize;
    split_until(&h, &mut boxes, &mut done, by_population, |b| {
        u64::from(b.count)
    });
    split_until(&h, &mut boxes, &mut done, max, |b| {
        u64::from(b.count) * b.volume()
    });
    boxes
        .into_iter()
        .chain(done)
        .map(|b| {
            let mut sum = [0u64; 3];
            for c in bins(b.lo, b.hi) {
                let s = h.sum[idx(c)];
                (0..3).for_each(|a| sum[a] += s[a]);
            }
            let n = u64::from(b.count);
            Swatch {
                rgb: sum.map(|s| ((s + n / 2) / n) as u8),
                population: b.count,
            }
        })
        .collect()
}
