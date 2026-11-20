//! 스타일 칩과 거르기 (1판 `style-of.ts`·`filter-icons.ts`·`recommended.ts`와 같은 규칙).

use crate::set::{Icon, IconSet};

/// 순서 있는 `(접미사, 이름)` 목록. 첫 항목이 기본 칩
pub type Styles = Vec<(String, String)>;

/// 세트의 스타일. 1판 추천 세트는 1판 이름·순서 그대로, 그 밖은 metadata suffixes를 `""` 먼저, 나머지는 키 순서로.
pub fn styles_of(set: &IconSet) -> Styles {
    let fixed: &[(&str, &str)] = match set.id.as_str() {
        "tabler" => &[("", "선"), ("filled", "채움")],
        "ri" => &[("line", "선"), ("fill", "채움")],
        "heroicons" => &[
            ("", "선 24"),
            ("solid", "채움 24"),
            ("20-solid", "채움 20"),
            ("16-solid", "채움 16"),
        ],
        // BTreeMap 순서라 `""`가 이미 맨 앞이다
        _ => return set.suffixes.clone().into_iter().collect(),
    };
    fixed
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// 이름 끝이 `-키`인 가장 긴 키. 없으면 `""`
pub fn style_of<'a>(name: &str, styles: &'a [(String, String)]) -> &'a str {
    let mut best = "";
    for (k, _) in styles {
        if k.len() > best.len()
            && name
                .strip_suffix(k.as_str())
                .is_some_and(|p| p.ends_with('-'))
        {
            best = k;
        }
    }
    best
}

/// 스타일(`None`이면 전체)과 검색어(공백으로 나눈 단어가 모두 이름에, 대소문자 무시)로 거른다.
/// 키가 1개 이하면 스타일로 거르지 않고, `""` 키가 없으면 접미사 없는 아이콘은 모든 스타일에 보인다.
pub fn filter<'a>(
    icons: &'a [Icon],
    query: &str,
    style: Option<&str>,
    styles: &[(String, String)],
) -> Vec<&'a Icon> {
    let query = query.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    let style = style.filter(|_| styles.len() > 1);
    let plain_everywhere = !styles.iter().any(|(k, _)| k.is_empty());
    icons
        .iter()
        .filter(|i| {
            style.is_none_or(|want| {
                let s = style_of(&i.name, styles);
                s == want || (plain_everywhere && s.is_empty())
            })
        })
        .filter(|i| {
            let name = i.name.to_lowercase();
            words.iter().all(|w| name.contains(w))
        })
        .collect()
}
