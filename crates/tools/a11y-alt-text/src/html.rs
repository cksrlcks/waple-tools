//! 출력 생성: ALT(한 줄, 영역 사이 ` | `) / SR-only(의미 구조 HTML).
//! SR-only는 내용(`sr_body`)과 숨김 스타일(`Hide`: class · inline)을 따로 만든다.

use crate::regions::{Kind, Region};

pub const SEPARATOR: &str = " | ";

/// HTML 글·속성 값 이스케이프
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 줄바꿈·탭·겹친 공백을 공백 하나로
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ALT 글: 영역(목록은 항목)마다 ` | `로 잇는 한 줄
pub fn alt_text(regions: &[Region]) -> String {
    regions
        .iter()
        .flat_map(|r| &r.items)
        .map(|s| one_line(s))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

pub fn img_tag(src: &str, alt: &str) -> String {
    format!("<img src=\"{}\" alt=\"{}\">", escape(src), escape(alt))
}

/// 화면에서만 숨기는 스타일 (display·visibility로 숨기면 스크린리더도 못 읽는다)
const SR_ONLY: [(&str, &str); 9] = [
    ("position", "absolute"),
    ("width", "1px"),
    ("height", "1px"),
    ("padding", "0"),
    ("margin", "-1px"),
    ("overflow", "hidden"),
    ("clip", "rect(0, 0, 0, 0)"),
    ("white-space", "nowrap"),
    ("border", "0"),
];

pub enum Hide<'a> {
    Class(&'a str),
    Inline,
}

/// 숨김 상자의 속성 (`class="…"` / `style="…"`)
fn hide_attr(hide: &Hide) -> String {
    match hide {
        Hide::Class(name) => format!("class=\"{}\"", escape(name)),
        Hide::Inline => {
            let decls: String = SR_ONLY
                .iter()
                .map(|(k, v)| format!("{k}:{};", v.replace(", ", ",")))
                .collect();
            format!("style=\"{decls}\"")
        }
    }
}

/// Class 방식의 CSS
pub fn css(class: &str) -> String {
    let decls: String = SR_ONLY
        .iter()
        .map(|(k, v)| format!("  {k}: {v};\n"))
        .collect();
    format!(".{class} {{\n{decls}}}\n")
}

/// 숨김 상자 안의 내용 (들여쓰기 두 칸). 제목은 `h{level}`, 문단은 `p`, 목록은 `ul`
pub fn sr_body(regions: &[Region], level: u8) -> String {
    let mut out = String::new();
    for r in regions {
        let items: Vec<String> = r
            .items
            .iter()
            .map(|s| escape(&one_line(s)))
            .filter(|s| !s.is_empty())
            .collect();
        if items.is_empty() {
            continue;
        }
        match r.kind {
            Kind::Title => out += &format!("  <h{level}>{}</h{level}>\n", items.join(" ")),
            Kind::Paragraph => out += &format!("  <p>{}</p>\n", items.join(" ")),
            Kind::List => {
                out += "  <ul>\n";
                for i in items {
                    out += &format!("    <li>{i}</li>\n");
                }
                out += "  </ul>\n";
            }
        }
    }
    out
}

/// SR-only HTML: 빈 alt 이미지(중복 읽기 방지) + 숨김 상자
pub fn sr_only_html(regions: &[Region], src: &str, level: u8, hide: &Hide) -> String {
    format!(
        "{}\n\n<div {}>\n{}</div>\n",
        img_tag(src, ""),
        hide_attr(hide),
        sr_body(regions, level)
    )
}

/// CSS 클래스 이름으로 쓸 수 있는가 (영문자·숫자·`-`·`_`, 숫자·`--`로 시작하지 않음)
pub fn valid_class(s: &str) -> bool {
    let t = s.strip_prefix('-').unwrap_or(s);
    t.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
