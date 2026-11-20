//! SVG · JSX · Data URI · 클래스 코드 (1판 `render-svg.ts`·`copy-formats.ts`와 같은 출력).

use std::fmt::Write as _;

use crate::set::{Icon, IconSet};

/// 아이콘 `(가로, 세로)`: 아이콘 값 → 세트 값
pub fn size_of(icon: &Icon, set: &IconSet) -> (f64, f64) {
    (
        icon.width.unwrap_or(set.width),
        icon.height.unwrap_or(set.height),
    )
}

/// `height=size`, `width=round(size×w/h)`. `stroke`가 있으면 모든 `stroke-width` 값을, `color`가
/// `currentColor`가 아니면 `currentColor`를 바꾼다. `color`는 호출하는 쪽이 검증한다.
pub fn svg(icon: &Icon, set: &IconSet, size: f64, stroke: Option<f64>, color: &str) -> String {
    let (w, h) = size_of(icon, set);
    let mut body = icon.body.clone();
    if let Some(s) = stroke {
        body = replace_stroke_width(&body, &s.to_string());
    }
    if color != "currentColor" {
        body = body.replace("currentColor", color);
    }
    let width = (size * w / h).round();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{size}" viewBox="0 0 {w} {h}">{body}</svg>"#
    )
}

/// `/stroke-width="[^"]*"/g` 바꾸기
fn replace_stroke_width(body: &str, value: &str) -> String {
    const KEY: &str = "stroke-width=\"";
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find(KEY) {
        let after = &rest[i + KEY.len()..];
        let Some(end) = after.find('"') else { break };
        out.push_str(&rest[..i]);
        let _ = write!(out, "{KEY}{value}\"");
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `/\s([a-z]+(?:-[a-z]+)+)=/g` → 공백 + camelCase 이름 (`data-`·`aria-`는 그대로)
pub fn jsx(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        if let Some(n) = c.is_whitespace().then(|| attr_len(after)).flatten() {
            let attr = &after[..n];
            if attr.starts_with("data-") || attr.starts_with("aria-") {
                out.push_str(&rest[..c.len_utf8() + n + 1]);
            } else {
                out.push(' ');
                for (i, part) in attr.split('-').enumerate() {
                    if i == 0 {
                        out.push_str(part);
                    } else {
                        out.push(part.as_bytes()[0].to_ascii_uppercase() as char);
                        out.push_str(&part[1..]);
                    }
                }
                out.push('=');
            }
            rest = &after[n + 1..];
            continue;
        }
        out.push(c);
        rest = after;
    }
    out
}

/// `s`가 `[a-z]+(-[a-z]+)+=`로 시작하면 `=` 앞까지 길이
fn attr_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let (mut i, mut parts) = (0, 0);
    loop {
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_lowercase) {
            i += 1;
        }
        if i == start {
            return None;
        }
        parts += 1;
        match b.get(i) {
            Some(b'-') => i += 1,
            Some(b'=') if parts >= 2 => return Some(i),
            _ => return None,
        }
    }
}

/// CSS `url("data:image/svg+xml,…")` (`encodeURIComponent`와 같은 인코딩)
pub fn data_uri(svg: &str) -> String {
    let mut out = String::from("url(\"data:image/svg+xml,");
    for b in svg.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out.push_str("\")");
    out
}

/// 웹폰트 클래스 규칙을 아는 세트만 (`tabler`·`ri`)
pub fn class_snippet(set_id: &str, name: &str) -> Option<String> {
    let class = match set_id {
        "tabler" => format!("ti ti-{name}"),
        "ri" => format!("ri-{name}"),
        _ => return None,
    };
    Some(format!(r#"<i class="{class}"></i>"#))
}
