//! 결과 블록 (Spec §6 순서): 세트 목록 → 아이콘 격자 또는 안내 → 고른 아이콘의 PNG·코드.

use std::collections::BTreeMap;
use std::path::PathBuf;

use base64::Engine as _;
use serde::Deserialize;
use waple_tools_core::{
    Block, GridCell, GridChips, GridChoice, GridMore, GridSearch, ResourceItem,
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

fn code(lang: &str, code: String, name: Option<String>) -> Block {
    Block::Code {
        lang: lang.into(),
        code,
        name,
    }
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
            .get_key_value(&o.style)
            .or_else(|| styles.first_key_value())
            .map(|(k, _)| k.as_str())
            .unwrap_or_default()
    });
    let icons = styles::filter(&set.icons, &o.query, style, &styles);
    let picked = icons
        .iter()
        .copied()
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

/// 패널로 가는 블록: PNG, SVG(저장), JSX, Data URI, 클래스
fn panel(o: &Options, set: &IconSet, icon: &Icon) -> Vec<Block> {
    let size = o
        .size
        .map(|s| f64::from(s.clamp(8, 1024)))
        .unwrap_or(render::size_of(icon, set).1);
    let stroke = o.stroke.map(|s| s.clamp(0.5, 3.0));
    // 컬러 세트는 색을 바꾸지 않는다
    let color = if o.use_color && !set.palette {
        if !valid_hex(&o.color) {
            return vec![text("색은 #rrggbb 형식이어야 합니다.")];
        }
        o.color.as_str()
    } else {
        "currentColor"
    };
    let svg = render::svg(icon, set, size, stroke, color);
    let name = &icon.name;
    let image = match png::render(&svg, o.png_scale.factor()) {
        Ok(bytes) => Block::Image {
            src: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
            name: format!("{name}.png"),
        },
        Err(e) => text(e.to_string()),
    };
    let (jsx, uri) = (render::jsx(&svg), render::data_uri(&svg));
    let mut out = vec![
        image,
        code("svg", svg, Some(format!("{name}.svg"))),
        code("jsx", jsx, None),
        code("data uri", uri, None),
    ];
    if let Some(c) = render::class_snippet(&set.id, name) {
        out.push(code("class", c, None));
    }
    out
}
