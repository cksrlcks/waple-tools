//! 명도대비 판단.
//!
//! - 글자: OCR(det)이 찾은 글자 줄 상자마다, **상자 가장자리**(글자 바로 바깥)의 최빈색을 배경으로 보고
//!   상자 안 가장 진한 픽셀과의 대비. 안티에일리어싱 중간색이나 회색으로만 그려진 가는 획(`ㅣ`)이 아니라
//!   획의 진한 색으로 판단한다. 가장자리마저 여러 색이면(사진·그라데이션 위 큰 제목) "배경 복잡"으로 둔다.
//! - 아이콘: 글자 상자 밖, 칸마다 배경색(최빈색)과 다른 색 픽셀의 연결 덩어리 중 **작고 색이 한두 가지인 것**만.
//!   그림(여러 색 · 큰 것)은 뺀다 (사용자: "아이콘은 잡아야 하는데 그림은 잡을 필요 없다"). 늘 3:1 (WCAG 1.4.11).

use crate::Crit;

/// 배경과 이보다 RGB 거리가 멀면 글자 픽셀
const INK_DIST: f32 = 48.0;
/// 최빈색이 이 비율보다 적으면 배경을 정하지 않는다 (사진 · 그라데이션)
const BG_SHARE: f32 = 0.3;
/// 아이콘 기준 (WCAG 1.4.11)
const ICON_NEED: f32 = 3.0;
/// 이보다 작은 덩어리는 잡티
const MIN_PIXELS: usize = 6;
/// 배경↔아이콘색 선분에서 이보다 먼(RGB 거리) 픽셀은 "다른 색"
const OFF_LINE: f32 = 40.0;
/// "다른 색" 픽셀이 이 비율을 넘으면 그림
const PICTURE_SHARE: f32 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug)]
pub(crate) struct Found {
    pub rect: Rect,
    /// 상자 안 가장 진한 획의 대비
    pub ratio: f32,
}

#[derive(Debug, Default)]
pub(crate) struct Report {
    /// 기준 미달 상자
    pub fails: Vec<Found>,
    /// 기준을 넘는 상자
    pub passes: Vec<Found>,
    /// 글자는 찾았지만 배경이 여러 색이라 대비를 정할 수 없는 상자
    pub unclear: Vec<Rect>,
}

fn linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 대비 `(밝은 쪽 + 0.05) / (어두운 쪽 + 0.05)`
pub(crate) fn ratio(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

struct Bg {
    rgb: [f32; 3],
    lum: f32,
}

fn lum_at(px: &[u8], lut: &[f32], i: usize) -> f32 {
    let p = &px[i * 4..i * 4 + 3];
    0.2126 * lut[p[2] as usize] + 0.7152 * lut[p[1] as usize] + 0.0722 * lut[p[0] as usize]
}

fn rgb_at(px: &[u8], i: usize) -> [f32; 3] {
    let p = &px[i * 4..i * 4 + 3];
    [p[2] as f32, p[1] as f32, p[0] as f32]
}

/// 채널당 4비트로 묶은 색 번호
fn key(c: [f32; 3]) -> usize {
    (c[0] as usize >> 4) << 8 | (c[1] as usize >> 4) << 4 | c[2] as usize >> 4
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn far(px: &[u8], i: usize, bg: &Bg) -> bool {
    dist(rgb_at(px, i), bg.rgb) > INK_DIST
}

/// 점 `p`에서 선분 `a`–`b`까지 (RGB)
fn off_segment(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / len2)
            .clamp(0.0, 1.0)
    };
    dist(p, [a[0] + t * ab[0], a[1] + t * ab[1], a[2] + t * ab[2]])
}

/// 색들의 최빈색(4비트로 묶은 칸)의 평균과 개수
fn mode(colors: impl Iterator<Item = [f32; 3]>) -> Option<([f32; 3], usize)> {
    let mut hist = std::collections::HashMap::<usize, (usize, [f32; 3])>::new();
    for c in colors {
        let e = hist.entry(key(c)).or_insert((0, [0.0; 3]));
        e.0 += 1;
        e.1 = [e.1[0] + c[0], e.1[1] + c[1], e.1[2] + c[2]];
    }
    let (_, (n, sum)) = hist.into_iter().max_by_key(|(_, (n, _))| *n)?;
    let f = n as f32;
    Some(([sum[0] / f, sum[1] / f, sum[2] / f], n))
}

/// `cells`(픽셀 번호들)의 최빈색 배경. 최빈색이 `BG_SHARE`보다 적으면 없음
fn background(px: &[u8], lut: &[f32], cells: impl Iterator<Item = usize> + Clone) -> Option<Bg> {
    let total = cells.clone().count();
    let (rgb, count) = mode(cells.clone().map(|i| rgb_at(px, i)))?;
    if (count as f32) < total as f32 * BG_SHARE {
        return None;
    }
    let k = key(rgb);
    let lsum: f32 = cells
        .filter(|&i| key(rgb_at(px, i)) == k)
        .map(|i| lum_at(px, lut, i))
        .sum();
    Some(Bg {
        rgb,
        lum: lsum / count as f32,
    })
}

/// 상자 픽셀 번호들
fn cells_of(r: Rect, w: usize) -> impl Iterator<Item = usize> + Clone {
    (r.y..r.y + r.h).flat_map(move |y| (r.x..r.x + r.w).map(move |x| y as usize * w + x as usize))
}

/// 글자 상자 하나의 대비: 상자 가장자리 띠의 최빈색 배경 대 상자 안 가장 진한 픽셀.
/// 배경을 정할 수 없으면(가장자리가 여러 색) `Err`, 글자 픽셀이 없으면 `Ok(None)`
fn box_ratio(px: &[u8], w: usize, lut: &[f32], r: Rect) -> Result<Option<f32>, ()> {
    // 가장자리 띠: 상자 높이의 1/6 (최소 2px). det 상자는 글자보다 조금 넓어(unclip) 띠는 대부분 배경
    let m = (r.h / 6).max(2);
    let ring = cells_of(r, w).filter(move |&i| {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        x < r.x + m || x >= r.x + r.w - m || y < r.y + m || y >= r.y + r.h - m
    });
    let bg = background(px, lut, ring).ok_or(())?;
    Ok(cells_of(r, w)
        .filter(|&i| far(px, i, &bg))
        .map(|i| ratio(lum_at(px, lut, i), bg.lum))
        .reduce(f32::max))
}

fn clip(r: &Rect, w: usize, h: usize) -> Rect {
    let (x0, y0) = (r.x.max(0), r.y.max(0));
    let (x1, y1) = ((r.x + r.w).min(w as i32), (r.y + r.h).min(h as i32));
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

/// `px`: 위에서부터 BGRA, `tile`: 칸 크기(px, 96 DPI 기준 24), `text`: OCR 글자 줄 상자(이미지 좌표).
/// 글자는 `crit.need()`, 글자 상자와 겹치지 않는 아이콘은 3:1
pub(crate) fn analyze(
    px: &[u8],
    w: usize,
    h: usize,
    tile: usize,
    crit: Crit,
    text: &[Rect],
) -> Report {
    assert!(px.len() >= w * h * 4);
    let lut: Vec<f32> = (0..=255).map(linear).collect();
    let mut report = Report::default();
    let mut judge = |found: Found, need: f32| {
        if found.ratio < need {
            report.fails.push(found);
        } else {
            report.passes.push(found);
        }
    };
    let mut unclear = Vec::new();
    for r in text {
        let rect = clip(r, w, h);
        if rect.w <= 0 || rect.h <= 0 {
            continue;
        }
        match box_ratio(px, w, &lut, rect) {
            Ok(Some(ratio)) => judge(Found { rect, ratio }, crit.need()),
            Ok(None) => {}
            Err(()) => unclear.push(rect),
        }
    }
    for f in icons(px, w, h, tile, &lut) {
        if !text.iter().any(|t| near(&f.rect, t, 0)) {
            judge(f, ICON_NEED);
        }
    }
    report.unclear = unclear;
    report
}

/// 아이콘: 배경과 다른 색 덩어리를 합친 상자 중 작고(긴 변 ≤ 칸의 8/3, 96 DPI에서 64px) 색이 한두 가지인 것
fn icons(px: &[u8], w: usize, h: usize, tile: usize, lut: &[f32]) -> Vec<Found> {
    let tile = tile.max(4);
    let (tw, th) = (w.div_ceil(tile), h.div_ceil(tile));
    let tile_of = |i: usize| (i / w / tile) * tw + (i % w) / tile;

    // 1. 칸마다 배경
    let mut bgs: Vec<Option<Bg>> = Vec::with_capacity(tw * th);
    for ty in 0..th {
        for tx in 0..tw {
            let (x0, y0) = (tx * tile, ty * tile);
            let (x1, y1) = ((x0 + tile).min(w), (y0 + tile).min(h));
            let cells = (y0..y1).flat_map(move |y| (x0..x1).map(move |x| y * w + x));
            bgs.push(background(px, lut, cells));
        }
    }

    // 2. 덩어리 픽셀: 그 칸 배경과 색이 다른 픽셀 (휘도가 같아도 색이 다르면 잡혀 낮은 대비로 나온다).
    //    이웃 칸의 배경색과 같으면 뺀다: 칸 경계에 걸친 카드·띠의 가장자리가 잡히지 않게
    let ink: Vec<bool> = (0..w * h)
        .map(|i| {
            if bgs[tile_of(i)].is_none() {
                return false;
            }
            let (tx, ty) = ((i % w) / tile, i / w / tile);
            (ty.saturating_sub(1)..=(ty + 1).min(th - 1)).all(|ny| {
                (tx.saturating_sub(1)..=(tx + 1).min(tw - 1))
                    .all(|nx| bgs[ny * tw + nx].as_ref().is_none_or(|bg| far(px, i, bg)))
            })
        })
        .collect();

    // 3. 8방향 연결 덩어리
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    let mut found = Vec::new();
    for start in 0..w * h {
        if !ink[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut count, mut best) = (0usize, 1f32);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            count += 1;
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            if let Some(bg) = &bgs[tile_of(i)] {
                best = best.max(ratio(lum_at(px, lut, i), bg.lum));
            }
            for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = ny * w + nx;
                    if ink[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        if count < MIN_PIXELS {
            continue;
        }
        let rect = Rect {
            x: x0 as i32,
            y: y0 as i32,
            w: (x1 - x0 + 1) as i32,
            h: (y1 - y0 + 1) as i32,
        };
        found.push((rect, best));
    }

    // 4. 가까운 덩어리를 합친 뒤 아이콘만: 알맞은 크기(96 DPI에서 긴 변 10~64px, 짧은 변 6px 이상 — 그림 조각·가는 선 제외),
    //    렌즈 가장자리에 걸려 잘리지 않고, 색이 한두 가지
    let t = tile as i32;
    let (max_long, min_long, min_short) = (t * 8 / 3, t * 5 / 12, t / 4);
    merge(found, t / 3)
        .into_iter()
        .filter(|f| {
            let r = f.rect;
            let (long, short) = (r.w.max(r.h), r.w.min(r.h));
            let cut = r.x == 0 || r.y == 0 || r.x + r.w >= w as i32 || r.y + r.h >= h as i32;
            (min_long..=max_long).contains(&long) && short >= min_short && !cut
        })
        .filter(|f| {
            let pts: Vec<([f32; 3], [f32; 3])> = cells_of(f.rect, w)
                .filter(|&i| ink[i])
                .filter_map(|i| bgs[tile_of(i)].as_ref().map(|bg| (rgb_at(px, i), bg.rgb)))
                .collect();
            flat(&pts)
        })
        .collect()
}

/// 덩어리 픽셀(색, 그 자리 배경색)이 "배경 ↔ 아이콘색(한두 가지)" 선분 근처에만 있나.
/// 안티에일리어싱 중간색은 선분 위에 있다. 여러 색이 섞이면(일러스트 · 사진 조각) 그림
fn flat(pts: &[([f32; 3], [f32; 3])]) -> bool {
    let Some(max_d) = pts.iter().map(|(p, bg)| dist(*p, *bg)).reduce(f32::max) else {
        return false;
    };
    // 아이콘색: 배경에서 가장 먼 쪽 픽셀(획의 진한 부분)의 최빈색, 그와 꽤 다른 둘째 색이 15% 넘으면 그것도
    let core: Vec<[f32; 3]> = pts
        .iter()
        .filter(|(p, bg)| dist(*p, *bg) >= max_d * 0.6)
        .map(|(p, _)| *p)
        .collect();
    let Some((fg1, _)) = mode(core.iter().copied()) else {
        return false;
    };
    let fg2 = mode(
        core.iter()
            .copied()
            .filter(|c| dist(*c, fg1) > OFF_LINE * 1.5),
    )
    .filter(|(_, n)| *n * 100 >= core.len() * 15)
    .map(|(c, _)| c);
    let off = pts
        .iter()
        .filter(|(p, bg)| {
            let mut d = off_segment(*p, *bg, fg1);
            if let Some(fg2) = fg2 {
                d = d
                    .min(off_segment(*p, *bg, fg2))
                    .min(off_segment(*p, fg1, fg2));
            }
            d > OFF_LINE
        })
        .count();
    (off as f32) <= pts.len() as f32 * PICTURE_SHARE
}

fn near(a: &Rect, b: &Rect, gap: i32) -> bool {
    a.x - gap < b.x + b.w && b.x - gap < a.x + a.w && a.y - gap < b.y + b.h && b.y - gap < a.y + a.h
}

/// 가까운 덩어리를 상자 하나로 (서로 이어지면 모두). 상자 대비는 가장 높은 값(가장 진한 획)
fn merge(items: Vec<(Rect, f32)>, gap: i32) -> Vec<Found> {
    let mut parent: Vec<usize> = (0..items.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..items.len() {
        for j in i + 1..items.len() {
            if near(&items[i].0, &items[j].0, gap) {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                parent[a] = b;
            }
        }
    }
    let mut out: Vec<(usize, Found)> = Vec::new();
    for (i, (r, v)) in items.iter().enumerate() {
        let k = root(&mut parent, i);
        match out.iter_mut().find(|(g, _)| *g == k) {
            Some((_, f)) => {
                let (x1, y1) = (
                    (f.rect.x + f.rect.w).max(r.x + r.w),
                    (f.rect.y + f.rect.h).max(r.y + r.h),
                );
                f.rect.x = f.rect.x.min(r.x);
                f.rect.y = f.rect.y.min(r.y);
                f.rect.w = x1 - f.rect.x;
                f.rect.h = y1 - f.rect.y;
                f.ratio = f.ratio.max(*v);
            }
            None => out.push((
                k,
                Found {
                    rect: *r,
                    ratio: *v,
                },
            )),
        }
    }
    out.into_iter().map(|(_, f)| f).collect()
}
