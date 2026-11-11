//! 색상 팔레트 추출 (추가 설치, Interactive): 이미지에서 주요 색 또는 역할별 색(Vibrant)을 뽑아
//! 색 칩 · CSS 변수 · JSON으로 보여 준다.

mod color;
mod mmcq;
#[cfg(test)]
mod tests;
mod vibrant;

use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{INPUT_EXTS, load_with, parse_options, schema_of};

use crate::color::{Rgb, body_text, hex, luminance, to_hsl};
use crate::mmcq::quantize;
use crate::vibrant::{ROLES, roles};

/// 계산 전에 긴 변을 이만큼으로 줄인다 (팔레트는 작은 그림으로 충분하다)
const SAMPLE_SIDE: u32 = 200;
/// 이보다 투명한 픽셀은 뺀다 (node-vibrant 기본 필터)
const MIN_ALPHA: u8 = 125;
/// 역할별에서 고를 후보 색 수 (node-vibrant 기본 maxColorCount)
const ROLE_CANDIDATES: usize = 64;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 주요 색상
    #[default]
    Dominant,
    /// 역할별 (Vibrant)
    Roles,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Sort {
    /// 비율순
    #[default]
    Ratio,
    /// 밝기순 (어두운 색부터)
    Brightness,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct PaletteOptions {
    /// 방식
    ///
    /// 주요 색상은 이미지에서 많이 쓰인 색을 고른 수만큼 뽑습니다. 역할별은 YouTube Music·Android가 앨범 커버에서 색을 고르는 방식(Android Palette)으로 선명한·어두운·밝은·차분한 색 6가지와 그 위에 쓸 글자색을 고릅니다. 역할별에서는 흰색에 가까운 픽셀을 뺍니다.
    mode: Mode,
    /// 색상 수
    #[schemars(range(min = 3, max = 12))]
    #[schemars(extend("x-enabled-when" = { "mode": ["dominant"] }))]
    count: u32,
    /// 정렬
    #[schemars(extend("x-enabled-when" = { "mode": ["dominant"] }))]
    sort: Sort,
}

impl Default for PaletteOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Dominant,
            count: 6,
            sort: Sort::Ratio,
        }
    }
}

struct MiscPalette;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscPalette)
}

waple_tools_addon_abi::export_tool!(tool);

/// 화면에 내놓을 색 하나
struct Entry {
    role: Option<&'static str>,
    rgb: Rgb,
    /// 0~1
    ratio: f64,
    /// 이미지에 없어 다른 역할의 색에서 만든 색
    derived: bool,
}

/// 줄인 이미지의 불투명 픽셀
fn sample(path: &Path) -> Result<Vec<Rgb>, ToolError> {
    // 움직이는 GIF는 첫 프레임에서 색을 뽑는다
    let img = load_with(path, true)?.image;
    // Nearest: 섞인 색을 새로 만들지 않고, 투명 픽셀이 가장자리에 번지지 않는다
    let img = if img.width().max(img.height()) > SAMPLE_SIDE {
        img.resize(SAMPLE_SIDE, SAMPLE_SIDE, FilterType::Nearest)
    } else {
        img
    };
    Ok(img
        .to_rgba8()
        .pixels()
        .filter(|p| p[3] >= MIN_ALPHA)
        .map(|p| [p[0], p[1], p[2]])
        .collect())
}

fn dominant(pixels: &[Rgb], count: usize, sort: Sort) -> Vec<Entry> {
    let mut sw = quantize(pixels, count);
    sw.sort_by_key(|s| std::cmp::Reverse(s.population));
    if sort == Sort::Brightness {
        sw.sort_by(|a, b| luminance(a.rgb).total_cmp(&luminance(b.rgb)));
    }
    let total = pixels.len() as f64;
    sw.iter()
        .map(|s| Entry {
            role: None,
            rgb: s.rgb,
            ratio: f64::from(s.population) / total,
            derived: false,
        })
        .collect()
}

/// 역할별 색과 못 찾은 역할 이름
fn by_role(pixels: &[Rgb]) -> (Vec<Entry>, Vec<&'static str>) {
    let total = pixels.len() as f64;
    let picks = roles(&quantize(pixels, ROLE_CANDIDATES));
    let (mut found, mut missing) = (vec![], vec![]);
    for (role, pick) in ROLES.into_iter().zip(picks) {
        match pick {
            Some(p) => found.push(Entry {
                role: Some(role),
                rgb: p.rgb,
                ratio: f64::from(p.population) / total,
                derived: p.derived,
            }),
            None => missing.push(role),
        }
    }
    (found, missing)
}

fn percent(ratio: f64) -> f64 {
    (ratio * 1000.0).round() / 10.0
}

/// [도, %, %]
fn hsl_int(c: Rgb) -> [u32; 3] {
    let [h, s, l] = to_hsl(c);
    [
        (h * 360.0).round() as u32 % 360,
        (s * 100.0).round() as u32,
        (l * 100.0).round() as u32,
    ]
}

/// `DarkVibrant` → `dark-vibrant`
fn kebab(role: &str) -> String {
    let mut out = String::new();
    for (i, ch) in role.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn blocks(entries: &[Entry]) -> Vec<Block> {
    let mut out = vec![];
    let mut css = String::from(":root {\n");
    let mut list = vec![];
    for (i, e) in entries.iter().enumerate() {
        let [r, g, b] = e.rgb;
        let hsl = hsl_int(e.rgb);
        let text = e.role.map(|_| hex(body_text(e.rgb)));
        let pct = format!("{:.1}%", percent(e.ratio));
        out.push(Block::Color {
            hex: hex(e.rgb),
            label: Some(match e.role {
                Some(role) if e.derived => format!("{role} · 만든 색"),
                Some(role) => format!("{role} · {pct}"),
                None => pct,
            }),
            text: text.clone(),
            detail: Some(format!(
                "rgb({r}, {g}, {b}) · hsl({}, {}%, {}%)",
                hsl[0], hsl[1], hsl[2]
            )),
        });
        let mut item = json!({
            "hex": hex(e.rgb),
            "rgb": e.rgb,
            "hsl": hsl,
            "percent": percent(e.ratio),
        });
        match (e.role, &text) {
            (Some(role), Some(text)) => {
                let name = kebab(role);
                css += &format!("  --{name}: {};\n  --{name}-text: {text};\n", hex(e.rgb));
                item["role"] = json!(role);
                item["bodyTextColor"] = json!(text);
                item["derived"] = json!(e.derived);
            }
            _ => css += &format!("  --palette-{}: {};\n", i + 1, hex(e.rgb)),
        }
        list.push(item);
    }
    css += "}\n";
    out.push(Block::Code {
        lang: "css".into(),
        code: css,
        name: Some("palette.css".into()),
    });
    out.push(Block::Code {
        lang: "json".into(),
        code: serde_json::to_string_pretty(&list).unwrap_or_default() + "\n",
        name: Some("palette.json".into()),
    });
    out
}

fn text(t: impl Into<String>) -> Block {
    Block::Text { text: t.into() }
}

fn palette(path: &Path, opts: &PaletteOptions) -> Result<Vec<Block>, ToolError> {
    let pixels = sample(path)?;
    if opts.mode == Mode::Dominant {
        if pixels.is_empty() {
            return Ok(vec![text("투명한 픽셀만 있어 색을 뽑을 수 없습니다.")]);
        }
        return Ok(blocks(&dominant(&pixels, opts.count as usize, opts.sort)));
    }
    // node-vibrant 기본 필터: 흰 배경이 LightMuted 등으로 뽑히지 않게
    let pixels: Vec<Rgb> = pixels
        .into_iter()
        .filter(|p| p.iter().any(|&v| v <= 250))
        .collect();
    if pixels.is_empty() {
        return Ok(vec![text(
            "투명하거나 흰색에 가까운 픽셀만 있어 역할별 색을 뽑을 수 없습니다.",
        )]);
    }
    let (found, missing) = by_role(&pixels);
    let mut out = vec![];
    if !missing.is_empty() {
        out.push(text(format!(
            "이 이미지에서 {} 역할에 맞는 색을 찾지 못해 뺐습니다.",
            missing.join(", ")
        )));
    }
    out.extend(blocks(&found));
    Ok(out)
}

impl Tool for MiscPalette {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.palette".into(),
            name: "색상 팔레트 추출".into(),
            category: "misc".into(),
            description: "이미지에서 주요 색상을 뽑아 HEX·RGB 코드로 보여 줍니다.".into(),
            extensions: INPUT_EXTS.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<PaletteOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: PaletteOptions = parse_options(opts)?;
        if !(3..=12).contains(&opts.count) {
            return Err(ToolError::InvalidOptions(
                "색상 수는 3~12 사이여야 합니다.".into(),
            ));
        }
        ctx.check_cancel()?;
        let path = input
            .first()
            .ok_or_else(|| ToolError::Failed("이미지를 넣어 주세요.".into()))?;
        palette(path, &opts).map(ToolOutput::Blocks)
    }
}
