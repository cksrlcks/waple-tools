//! 결과 블록 (Spec §6 순서): 세트 목록 → 아이콘 격자 또는 안내 → 고른 아이콘의 버튼 묶음.

use std::collections::BTreeMap;
use std::path::PathBuf;

use base64::Engine as _;
use serde::Deserialize;
use waple_tools_core::{
    Action, ActionItem, Block, GridCell, GridChips, GridChoice, GridMore, GridSearch, ResourceItem,
};

use crate::set::{self, Icon, IconSet};
use crate::{Options, collections, png, render, styles};

/// [더 보기]마다 늘리는 개수
pub const STEP: u32 = 400;
const NOT_DOWNLOADED: &str = "목록에서 [받기]를 누르면 아이콘을 볼 수 있습니다.";

/// 앱이 넣어 주는 `__resources`: 받은 세트 폴더와 `{id: version}`
#[derive(Deserialize)]
pub struct Resources {
    pub dir: PathBuf,
    #[serde(default)]
    pub installed: BTreeMap<String, String>,
}

fn text(t: impl Into<String>) -> Block {
    Block::Text { text: t.into() }
}

/// 1234567 → "1,234,567"
fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn resource_list(o: &Options) -> Block {
    Block::ResourceList {
        items: collections::all()
            .iter()
            .map(|c| ResourceItem {
                id: c.id.clone(),
                title: c.name.clone(),
                meta: format!("{}개 · {}", thousands(c.total), c.license),
            })
            .collect(),
        option: "set".into(),
        resets: ["selected", "query", "style", "limit"]
            .map(Into::into)
            .into(),
        selected: (!o.set.is_empty()).then(|| o.set.clone()),
    }
}

pub fn build(o: &Options, res: Option<&Resources>) -> Vec<Block> {
    let mut blocks = vec![resource_list(o)];
    if o.set.is_empty() {
        return blocks;
    }
    let Some((res, version)) = res.and_then(|r| Some((r, r.installed.get(&o.set)?))) else {
        blocks.push(text(NOT_DOWNLOADED));
        return blocks;
    };
    let set = match set::load(&res.dir, &o.set, version) {
        Ok(s) => s,
        Err(e) => {
            blocks.push(text(e));
            return blocks;
        }
    };
    let styles = styles::styles_of(&set);
    // 키가 1개 이하면 칩 없음. 고른 값이 키가 아니면(처음·다른 세트) 첫 키
    let style = (styles.len() > 1).then(|| {
        styles
            .iter()
            .find(|(k, _)| *k == o.style)
            .unwrap_or(&styles[0])
            .0
            .as_str()
    });
    let icons = styles::filter(&set.icons, &o.query, style, &styles);
    // 검색·칩으로 격자에서 가려져도 패널은 유지한다 (세트 전체에서 찾는다)
    let picked = set
        .icons
        .iter()
        .find(|i| !o.selected.is_empty() && i.name == o.selected);
    let total = icons.len() as u32;
    blocks.push(Block::IconGrid {
        cells: icons
            .iter()
            .take(o.limit as usize)
            .map(|i| GridCell {
                key: i.name.clone(),
                svg: render::svg(i, &set, 24.0, None, "currentColor"),
                title: i.name.clone(),
            })
            .collect(),
        option: "selected".into(),
        selected: picked.map(|i| i.name.clone()),
        chips: style.map(|value| GridChips {
            option: "style".into(),
            choices: styles
                .iter()
                .map(|(k, v)| GridChoice {
                    value: k.clone(),
                    label: v.clone(),
                })
                .collect(),
            value: value.into(),
            resets: vec!["limit".into()],
        }),
        search: Some(GridSearch {
            option: "query".into(),
            value: o.query.clone(),
            placeholder: "아이콘 검색".into(),
            resets: vec!["limit".into()],
        }),
        more: (total > o.limit).then(|| GridMore {
            option: "limit".into(),
            value: o.limit.saturating_add(STEP),
            label: "더 보기".into(),
        }),
        total,
    });
    if let Some(icon) = picked {
        blocks.extend(panel(o, &set, icon));
    }
    blocks
}

fn valid_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// 패널로 가는 블록: 버튼 묶음 하나 (SVG·PNG·JSX·Data URI·클래스 복사, SVG 저장)
fn panel(o: &Options, set: &IconSet, icon: &Icon) -> Vec<Block> {
    let px = o.size.clamp(8, 512);
    let stroke = o.stroke.map(|s| s.clamp(0.5, 3.0));
    // 컬러 세트는 색을 바꾸지 않는다
    let color = if set.palette {
        "currentColor"
    } else if valid_hex(&o.color) {
        o.color.as_str()
    } else {
        return vec![text("색은 #rrggbb 형식이어야 합니다.")];
    };
    let svg = render::svg(icon, set, f64::from(px), stroke, color);
    let name = &icon.name;
    let item = |label: &str, action| ActionItem {
        label: label.into(),
        action,
    };
    let copy = |label: &str, t: String| item(label, Action::CopyText { text: t });
    let mut items = vec![copy("SVG 복사", svg.clone())];
    match png::render(&svg, 1) {
        Ok(bytes) => items.push(item(
            "PNG 복사",
            Action::CopyImage {
                src: format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ),
            },
        )),
        Err(e) => return vec![text(e.to_string())],
    }
    items.push(copy("JSX 복사", render::jsx(&svg)));
    items.push(copy("Data URI 복사", render::data_uri(&svg)));
    if let Some(c) = render::class_snippet(&set.id, name) {
        items.push(copy("클래스 복사", c));
    }
    items.push(item(
        "SVG 저장",
        Action::SaveText {
            text: svg,
            name: format!("{name}.svg"),
        },
    ));
    vec![Block::Actions { items }]
}
