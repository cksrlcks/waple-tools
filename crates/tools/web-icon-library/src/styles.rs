//! 스타일 칩과 거르기 (1판 `style-of.ts`·`filter-icons.ts`와 같은 규칙).

use std::collections::BTreeMap;

use crate::set::{Icon, IconSet};

/// 세트의 스타일 `{접미사: 이름}`. `tabler`는 metadata에 없어 여기서 정한다.
pub fn styles_of(set: &IconSet) -> BTreeMap<String, String> {
    if set.id == "tabler" {
        return [("", "선"), ("filled", "채움")]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into();
    }
    set.suffixes.clone()
}

/// 이름 끝이 `-키`인 가장 긴 키. 없으면 `""`
pub fn style_of<'a>(name: &str, styles: &'a BTreeMap<String, String>) -> &'a str {
    let mut best = "";
    for k in styles.keys() {
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
    styles: &BTreeMap<String, String>,
) -> Vec<&'a Icon> {
    let query = query.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    let style = style.filter(|_| styles.len() > 1);
    let plain_everywhere = !styles.contains_key("");
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
