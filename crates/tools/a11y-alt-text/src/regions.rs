//! 정보 영역과 대체텍스트 칸(영역 사이 ` | `)의 해석

use crate::layout::join_text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Title,
    Paragraph,
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub kind: Kind,
    /// 제목·문단은 글 하나, 목록은 항목들
    pub items: Vec<String>,
    /// 이 영역을 이룬 OCR 줄 번호 (직접 수정한 영역은 비어 있다)
    pub lines: Vec<usize>,
}

/// 대체텍스트 칸(영역 사이 `|`) → 영역. 줄바꿈·겹친 공백은 공백 하나로.
/// 자동 분석의 제목·목록 항목과 글이 같으면 그 구조(heading·ul)를 이어받고, 나머지는 문단
pub fn from_alt(text: &str, auto: &[Region]) -> Vec<Region> {
    let mut out: Vec<Region> = Vec::new();
    // 바로 앞 조각이 속한 자동 목록 (이어지는 항목을 한 목록으로)
    let mut list: Option<usize> = None;
    for part in text.split('|') {
        let t = join_text([part]);
        if t.is_empty() {
            list = None;
            continue;
        }
        let hit = auto.iter().position(|r| r.items.contains(&t));
        match hit.map(|k| (k, auto[k].kind)) {
            Some((k, Kind::List)) => {
                match out.last_mut() {
                    Some(last) if list == Some(k) => last.items.push(t),
                    _ => out.push(region(Kind::List, t)),
                }
                list = Some(k);
                continue;
            }
            Some((_, Kind::Title)) => out.push(region(Kind::Title, t)),
            _ => out.push(region(Kind::Paragraph, t)),
        }
        list = None;
    }
    out
}

fn region(kind: Kind, text: String) -> Region {
    Region {
        kind,
        items: vec![text],
        lines: Vec::new(),
    }
}

/// 영역 카드 이름표 (화면 표시용. 출력 HTML 구조는 `kind`만 쓴다)
pub fn label(r: &Region) -> &'static str {
    let t = r.items.join(" ");
    let t = t.trim_start();
    match r.kind {
        Kind::Title => "제목",
        Kind::List => "목록",
        _ if t.starts_with(['※', '*', '＊']) || t.contains("주의") || t.contains("유의") => {
            "주의사항"
        }
        _ if t.starts_with("문의") || t.contains('@') || has_phone(t) => "문의처",
        _ if t.starts_with("장소") || t.starts_with("위치") => "장소",
        _ if t.starts_with("신청") || t.starts_with("접수") => "신청 방법",
        _ if has_date_time(t) => "날짜·시간",
        _ => "설명",
    }
}

/// `2026년`·`11월`·`17일`·`17:30` 같은 날짜·시간
pub(crate) fn has_date_time(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    c.windows(2)
        .any(|w| w[0].is_ascii_digit() && matches!(w[1], '년' | '월' | '일'))
        || c.windows(3)
            .any(|w| w[0].is_ascii_digit() && w[1] == ':' && w[2].is_ascii_digit())
}

/// `02-123-4567`·`010-1234-5678` 전화번호
pub(crate) fn has_phone(t: &str) -> bool {
    t.split(|c: char| !(c.is_ascii_digit() || c == '-'))
        .any(|tok| {
            let parts: Vec<usize> = tok.split('-').map(str::len).collect();
            matches!(parts[..], [a, b, 4] if (2..=4).contains(&a) && (3..=4).contains(&b))
        })
}
