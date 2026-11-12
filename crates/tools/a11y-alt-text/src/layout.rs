//! OCR 후처리: 같은 줄 조각 잇기 → 읽기 순서(XY-cut) → 문장 병합·정보 영역 나누기.
//! 글은 OCR이 읽은 그대로 쓰고 줄바꿈·겹친 공백만 정리한다(고치거나 지어내지 않는다).

use crate::regions::{Kind, Region};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn w(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn h(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn union(&self, o: &Rect) -> Rect {
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    fn v_overlap(&self, o: &Rect) -> f32 {
        (self.y1.min(o.y1) - self.y0.max(o.y0)).max(0.0)
    }

    fn h_overlap(&self, o: &Rect) -> f32 {
        self.x1.min(o.x1) - self.x0.max(o.x0)
    }
}

/// OCR 원본 한 줄: 읽은 글, 신뢰도(0~1), 이미지 좌표 상자
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub confidence: f32,
    pub rect: Rect,
}

/// 같은 줄 조각 사이 빈틈 상한 (글자 높이 배수). 넘으면 다른 칸·단으로 본다
const ROW_GAP: f32 = 1.2;
/// 같은 문단의 줄 간격 상한 (글자 높이 배수)
const PARA_GAP: f32 = 0.7;
/// 문장이 끝나지 않았을 때 이어 붙이는 줄 간격 상한
const SENTENCE_GAP: f32 = 1.6;
/// 이만큼 글자 높이가 다르면 다른 영역 (제목·본문)
const SIZE_STEP: f32 = 1.4;
/// 중간 글자 높이의 이 배수 이상이면 제목
const TITLE_RATIO: f32 = 1.3;
const TITLE_MAX_CHARS: usize = 40;

/// det이 넓은 띄어쓰기에서 끊은 같은 줄 조각을 한 줄로 잇는다
pub fn join_rows(mut frags: Vec<Line>) -> Vec<Line> {
    frags.sort_by(|a, b| a.rect.x0.total_cmp(&b.rect.x0));
    let mut rows: Vec<Line> = Vec::new();
    for f in frags {
        let hit = rows.iter_mut().find(|r| {
            let (a, b) = (r.rect.h(), f.rect.h());
            let min = a.min(b);
            let gap = f.rect.x0 - r.rect.x1;
            r.rect.v_overlap(&f.rect) >= 0.6 * min
                && a.max(b) / min <= SIZE_STEP
                && gap >= -0.5 * min
                && gap <= ROW_GAP * min
        });
        match hit {
            Some(r) => {
                r.text = format!("{} {}", r.text, f.text);
                r.confidence = r.confidence.min(f.confidence);
                r.rect = r.rect.union(&f.rect);
            }
            None => rows.push(f),
        }
    }
    rows
}

/// 읽기 순서 (줄 번호 목록). 재귀 XY-cut: 가로·세로 빈틈 중 가장 넓은 곳에서 자르고
/// 위→아래, 왼쪽→오른쪽으로 읽는다. 다단은 단 사이 빈틈이 줄 간격보다 넓어 단끼리 먼저 묶인다
pub fn reading_order(lines: &[Line]) -> Vec<usize> {
    let mut out = Vec::with_capacity(lines.len());
    xy_cut(lines, (0..lines.len()).collect(), &mut out);
    out
}

fn xy_cut(lines: &[Line], idx: Vec<usize>, out: &mut Vec<usize>) {
    // 위아래 줄 상자가 살짝 겹쳐도 빈틈을 찾게 높이의 10%씩 줄여서 본다
    let ys = |i: usize| {
        let r = lines[i].rect;
        (r.y0 + r.h() * 0.1, r.y1 - r.h() * 0.1)
    };
    let xs = |i: usize| (lines[i].rect.x0, lines[i].rect.x1);
    let y = widest_gap(&idx, ys);
    let x = widest_gap(&idx, xs);
    let cut = match (y, x) {
        (Some(y), Some(x)) if x.1 > y.1 => Some((false, x.0)),
        (Some(y), _) => Some((true, y.0)),
        (None, Some(x)) => Some((false, x.0)),
        (None, None) => None,
    };
    match cut {
        Some((horizontal, at)) => {
            let (a, b): (Vec<usize>, Vec<usize>) = idx
                .into_iter()
                .partition(|&i| if horizontal { ys(i).0 } else { xs(i).0 } < at);
            xy_cut(lines, a, out);
            xy_cut(lines, b, out);
        }
        None => {
            let mut idx = idx;
            let cy = |i: usize| (lines[i].rect.y0 + lines[i].rect.y1) / 2.0;
            idx.sort_by(|&a, &b| {
                cy(a)
                    .total_cmp(&cy(b))
                    .then(lines[a].rect.x0.total_cmp(&lines[b].rect.x0))
            });
            out.extend(idx);
        }
    }
}

/// 구간들을 한 축에 겹쳐 놓았을 때 가장 넓은 빈틈: (빈틈 가운데, 폭)
fn widest_gap(idx: &[usize], span: impl Fn(usize) -> (f32, f32)) -> Option<(f32, f32)> {
    let mut s: Vec<(f32, f32)> = idx.iter().map(|&i| span(i)).collect();
    s.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut end = s.first()?.1;
    let mut best: Option<(f32, f32)> = None;
    for &(a, b) in &s[1..] {
        if a > end && best.is_none_or(|(_, w)| a - end > w) {
            best = Some(((a + end) / 2.0, a - end));
        }
        end = end.max(b);
    }
    best
}

/// 읽기 순서대로 줄을 영역으로 묶는다
pub fn regions(lines: &[Line], order: &[usize]) -> Vec<Region> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if !breaks(&lines[g[0]], &lines[order[k - 1]], &lines[i]) => g.push(i),
            _ => groups.push(vec![i]),
        }
    }
    let mut heights: Vec<f32> = lines.iter().map(|l| l.rect.h()).collect();
    heights.sort_by(f32::total_cmp);
    let median = heights.get(heights.len() / 2).copied().unwrap_or(0.0);
    groups
        .into_iter()
        .map(|g| build(lines, g, median))
        .collect()
}

/// `prev` 다음 줄 `cur`가 새 영역을 시작하는가. `first`는 지금 영역의 첫 줄
fn breaks(first: &Line, prev: &Line, cur: &Line) -> bool {
    let (p, c) = (prev.rect, cur.rect);
    let h = p.h().min(c.h());
    if p.h().max(c.h()) / h > SIZE_STEP || p.h_overlap(&c) <= 0.0 {
        return true; // 글자 크기가 다르거나 다른 단
    }
    let gap = c.y0 - p.y1;
    // 목록 기호로 시작한 영역의 다음 기호 줄은 같은 목록의 항목
    let next_item = is_bullet(&first.text) && is_bullet(&cur.text);
    if gap < -0.5 * h || (starts_item(&cur.text) && !next_item) {
        return true; // 위로 올라감(다른 단) 또는 항목·이름표로 시작
    }
    if gap <= PARA_GAP * h {
        return false;
    }
    // 문단 간격만큼 떨어져도 앞 문장이 끝나지 않았으면 이어지는 문장
    gap > SENTENCE_GAP * h || ends_sentence(&prev.text)
}

const BULLETS: &[char] = &[
    '•', '·', '■', '□', '●', '○', '◆', '◇', '▶', '►', '▷', '-', '–', '➤', '✔', '✓',
];
const KEYWORDS: &[&str] = &[
    "문의", "신청", "접수", "장소", "일시", "기간", "주의", "유의",
];

fn is_bullet(t: &str) -> bool {
    t.starts_with(BULLETS)
}

fn strip_bullet(t: &str) -> &str {
    t.trim_start_matches(BULLETS).trim_start()
}

/// 목록 기호·참고 기호·번호·이름표(`장소:`)·안내 낱말로 시작하는 줄
pub(crate) fn starts_item(t: &str) -> bool {
    let t = t.trim_start();
    if is_bullet(t) || t.starts_with(['※', '*', '＊']) || KEYWORDS.iter().any(|k| t.starts_with(k))
    {
        return true;
    }
    // `1.` `2)` 번호
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if (1..=2).contains(&digits) && t[digits..].starts_with(['.', ')']) {
        return true;
    }
    // `점검시간:` 이름표 (숫자 뒤 쌍점 `17:30`은 아님)
    match t.find([':', '：']) {
        Some(i) => {
            let label = &t[..i];
            !label.trim().is_empty()
                && label.chars().count() <= 10
                && !label.chars().any(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// 문장이 끝난 줄: 마침표류, 또는 `~다`·`~요`로 끝남 (배너는 마침표를 자주 뺀다)
pub(crate) fn ends_sentence(t: &str) -> bool {
    t.trim_end().ends_with(['.', '!', '?', '。', '다', '요'])
}

/// 줄들을 공백 하나로 잇는다 (디자인 때문에 끊긴 줄바꿈 복원, 겹친 공백 정리)
pub(crate) fn join_text<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    parts
        .into_iter()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// OCR이 자주 빠뜨리는 띄어쓰기만 채운다(뜻은 그대로, 원문은 카드에 남는다):
/// `이름표:값`, 숫자 뒤 년·월·일 다음 숫자, 한글 사이 문장부호 뒤, `※` 뒤.
/// URL(`https://`)·시간(`17:30`)·소수·영문 마침표(`example.com`)는 건드리지 않는다
pub(crate) fn tidy_spacing(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let hangul = |x: char| ('가'..='힣').contains(&x);
    let mut out = String::with_capacity(s.len() + 8);
    for (i, &ch) in c.iter().enumerate() {
        out.push(ch);
        let prev = i.checked_sub(1).map(|j| c[j]);
        let Some(&next) = c.get(i + 1) else { continue };
        let space = match ch {
            ':' | '：' => {
                prev.is_some_and(|p| !p.is_ascii_digit() && !p.is_whitespace())
                    && next.is_alphanumeric()
            }
            '.' | '!' | '?' => prev.is_some_and(hangul) && hangul(next),
            '년' | '월' | '일' => {
                prev.is_some_and(|p| p.is_ascii_digit()) && next.is_ascii_digit()
            }
            '※' => !next.is_whitespace(),
            _ => false,
        };
        if space {
            out.push(' ');
        }
    }
    out
}

fn build(lines: &[Line], g: Vec<usize>, median: f32) -> Region {
    let texts: Vec<&str> = g.iter().map(|&i| lines[i].text.as_str()).collect();
    if texts.iter().filter(|t| is_bullet(t)).count() >= 2 {
        // 목록: 기호로 시작하는 줄이 항목, 기호 없는 줄은 앞 항목에 이어 붙인다. 기호는 장식이라 뺀다
        let mut items: Vec<String> = Vec::new();
        for t in texts {
            match items.last_mut() {
                Some(last) if !is_bullet(t) => *last = join_text([last.as_str(), t]),
                _ => items.push(join_text([strip_bullet(t)])),
            }
        }
        return Region {
            kind: Kind::List,
            items: items.iter().map(|i| tidy_spacing(i)).collect(),
            lines: g,
        };
    }
    let text = tidy_spacing(&join_text(texts));
    let tall = g.iter().all(|&i| lines[i].rect.h() >= TITLE_RATIO * median);
    let kind = if tall && text.chars().count() <= TITLE_MAX_CHARS {
        Kind::Title
    } else {
        Kind::Paragraph
    };
    Region {
        kind,
        items: vec![text],
        lines: g,
    }
}

/// OCR 조각 → 영역 (조각 잇기 · 읽기 순서 · 영역 나누기). 줄은 읽기 순서로 다시 놓는다
pub fn analyze(frags: Vec<Line>) -> (Vec<Line>, Vec<Region>) {
    let rows = join_rows(frags);
    let order = reading_order(&rows);
    let lines: Vec<Line> = order.iter().map(|&i| rows[i].clone()).collect();
    let order: Vec<usize> = (0..lines.len()).collect();
    let regions = regions(&lines, &order);
    (lines, regions)
}
