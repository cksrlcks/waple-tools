//! 대체텍스트 (추가 설치, Interactive, AI): 안내 이미지의 글자를 PaddleOCR로 읽어 정보 영역으로 나누고
//! ALT(영역 사이 ` | `) 또는 SR-only HTML(Inline CSS · Class)을 만든다. 모두 이 PC에서 처리한다.
//!
//! 책임: `ocr`(추론) → `layout`(읽기 순서·문장 병합·영역) → `regions`(영역·대체텍스트 칸 해석) → `html`(출력).
//! OCR 결과는 파일별로 기억해 두어 출력 옵션을 바꿔도 다시 읽지 않는다.

mod html;
mod layout;
mod ocr;
mod overlay;
mod regions;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use waple_tools_core::{
    Block, JobContext, Record, RecordField, Tool, ToolError, ToolKind, ToolMeta, ToolOutput,
};
use waple_tools_image_common::{load, parse_options, schema_of};

use crate::html::Hide;
use crate::layout::Line;
use crate::regions::Region;

/// 이보다 낮은 신뢰도의 줄은 '확인 필요'
const LOW_CONFIDENCE: f32 = 0.85;
pub const INPUT_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp"];

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Output {
    /// ALT 방식
    #[default]
    Alt,
    /// SR-only 방식
    SrOnly,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum StyleMode {
    /// Class 사용
    #[default]
    Class,
    /// Inline CSS
    Inline,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Heading {
    /// h1
    H1,
    /// h2
    #[default]
    H2,
    /// h3
    H3,
    /// h4
    H4,
    /// h5
    H5,
    /// h6
    H6,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
// 결과 코드(img·HTML·CSS)는 옵션 패널 아래에, 작업대에는 영역 이미지·카드만
#[schemars(extend("x-result-panel" = true))]
struct AltOptions {
    /// 출력 방식
    ///
    /// ALT는 img의 alt 속성에 넣을 한 줄 글로, 서로 다른 정보 영역 사이에 ' | '를 넣습니다. ' | '를 읽는 방식은 스크린리더마다 다르니 실제로 들어 확인하세요. SR-only는 화면에서는 숨기고 스크린리더만 읽는 HTML입니다. 바꿔도 글자를 다시 읽지 않습니다.
    output: Output,
    /// 대체텍스트
    ///
    /// 이미지를 읽으면 자동으로 채워집니다. 서로 다른 정보 영역 사이에 ' | '가 들어갑니다. 고치면 img·SR-only HTML에 바로 반영됩니다. ' | '를 넣으면 영역이 나뉘고, 지우면 합쳐집니다. 줄바꿈은 공백이 됩니다. 모두 지우면 자동 분석 글로 돌아갑니다.
    // 앱이 결과의 'ALT' 코드 블록으로 빈 칸을 채우고, 그 블록은 결과에서 숨긴다.
    // SR-only에서는 칸을 숨기고 HTML·CSS 코드를 보인다 (값은 남아 SR-only HTML에도 쓰인다)
    #[schemars(extend(
        "x-multiline" = true,
        "x-fill-from" = "ALT",
        "x-copy" = true,
        "x-enabled-when" = { "output": ["alt"] }
    ))]
    text: String,
    /// 제목 수준
    #[schemars(extend("x-enabled-when" = { "output": ["srOnly"] }))]
    heading: Heading,
    /// CSS 방식
    ///
    /// Class는 HTML에 클래스만 넣고 CSS를 따로 줍니다. Inline CSS는 숨김 스타일을 style 속성에 직접 써서 HTML만 복사해도 됩니다.
    #[schemars(extend("x-enabled-when" = { "output": ["srOnly"] }))]
    style_mode: StyleMode,
    /// 클래스명
    #[schemars(extend("x-enabled-when" = { "output": ["srOnly"], "styleMode": ["class"] }))]
    class_name: String,
}

impl Default for AltOptions {
    fn default() -> Self {
        Self {
            output: Output::Alt,
            heading: Heading::H2,
            style_mode: StyleMode::Class,
            class_name: "sr-only".into(),
            text: String::new(),
        }
    }
}

/// 이미지 한 장의 OCR · 자동 영역 · 영역 표시 이미지
struct Analysis {
    lines: Vec<Line>,
    regions: Vec<Region>,
    overlay: String,
}

type Key = (PathBuf, u64, Option<SystemTime>);

/// 마지막으로 읽은 이미지 하나만 기억한다 (같은 파일·크기·수정 시각이면 다시 읽지 않는다)
static CACHE: Mutex<Option<(Key, Arc<Analysis>)>> = Mutex::new(None);

fn analyze(ctx: &JobContext, path: &Path) -> Result<Arc<Analysis>, ToolError> {
    let m = std::fs::metadata(path)?;
    let key: Key = (path.to_path_buf(), m.len(), m.modified().ok());
    // 분석하는 동안 잠가 둔다: 처음 OCR 중에 옵션을 바꿔 다시 불려도 같은 이미지를 두 번 읽지 않고 결과를 기다린다
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, a)) = cache.as_ref()
        && *k == key
    {
        return Ok(a.clone());
    }
    let img = load(path)?.image.to_rgb8();
    let frags = ocr::recognize(ctx, &img)?;
    let (lines, regions) = layout::analyze(frags);
    let overlay = overlay::render(&img, &lines, &regions)?;
    let a = Arc::new(Analysis {
        lines,
        regions,
        overlay,
    });
    *cache = Some((key, a.clone()));
    Ok(a)
}

fn percent(c: f32) -> String {
    format!("{}%", (c * 100.0).round() as u32)
}

/// 영역 카드: 정리된 글과 그 영역을 이룬 OCR 원문 줄(신뢰도)
fn cards(a: &Analysis) -> Vec<Record> {
    a.regions
        .iter()
        .enumerate()
        .map(|(n, r)| {
            let low = r
                .lines
                .iter()
                .map(|&i| a.lines[i].confidence)
                .fold(1.0f32, f32::min);
            let text = match r.kind {
                regions::Kind::List => r
                    .items
                    .iter()
                    .map(|i| format!("- {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => r.items.join(" "),
            };
            Record {
                title: format!("{} · {}", n + 1, regions::label(r)),
                meta: Some(if low < LOW_CONFIDENCE {
                    format!("확인 필요 · 신뢰도 {}", percent(low))
                } else {
                    format!("신뢰도 {}", percent(low))
                }),
                text,
                fields: r
                    .lines
                    .iter()
                    .map(|&i| RecordField {
                        label: format!("OCR {}", percent(a.lines[i].confidence)),
                        value: a.lines[i].text.clone(),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn code(lang: &str, code: String) -> Block {
    Block::Code {
        lang: lang.into(),
        code,
        name: None,
    }
}

fn blocks(a: &Analysis, o: &AltOptions, src: &str) -> Vec<Block> {
    if a.lines.is_empty() {
        return vec![Block::Text {
            text: "이미지에서 글자를 찾지 못했습니다.".into(),
        }];
    }
    let regions = if o.text.trim().is_empty() {
        a.regions.clone()
    } else {
        regions::from_alt(&o.text, &a.regions)
    };
    let mut out = Vec::new();
    let low = a
        .lines
        .iter()
        .filter(|l| l.confidence < LOW_CONFIDENCE)
        .count();
    if low > 0 {
        out.push(Block::Text {
            text: format!(
                "신뢰도가 낮은 줄이 {low}개 있습니다('확인 필요' 카드). 이미지와 비교해 '대체텍스트' 칸에서 고쳐 주세요."
            ),
        });
    }
    out.push(Block::Image {
        src: a.overlay.clone(),
        // 확인용 그림이라 [이미지 복사]·[저장]은 두지 않는다
        name: String::new(),
    });
    out.push(Block::Records {
        items: cards(a),
        // 카드 [복사]: ALT 형식 한 줄 (영역 사이 ` | `)
        copy: Some(html::alt_text(&regions)),
    });
    // 대체텍스트 칸을 채울 글 (앱은 이 블록을 칸에 넣고 결과에서는 숨긴다)
    let alt = html::alt_text(&regions);
    out.push(code("ALT", alt));
    match o.output {
        Output::Alt => {}
        Output::SrOnly => {
            let level = o.heading as u8 + 1;
            match o.style_mode {
                StyleMode::Inline => {
                    out.push(code(
                        "HTML",
                        html::sr_only_html(&regions, src, level, &Hide::Inline),
                    ));
                }
                StyleMode::Class => {
                    let class = o.class_name.trim();
                    let markup = html::sr_only_html(&regions, src, level, &Hide::Class(class));
                    let css = html::css(class);
                    out.push(code("HTML", markup.clone()));
                    out.push(code("CSS", css.clone()));
                    out.push(code(
                        "HTML + CSS",
                        format!("<style>\n{css}</style>\n\n{markup}"),
                    ));
                }
            }
        }
    }
    out
}

struct AltText;

pub fn tool() -> Box<dyn Tool> {
    Box::new(AltText)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for AltText {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "a11y.alt_text".into(),
            name: "대체텍스트".into(),
            category: "a11y".into(),
            description: "이미지 글자로 대체텍스트를 만듭니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<AltOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: AltOptions = parse_options(opts)?;
        if o.output == Output::SrOnly
            && o.style_mode == StyleMode::Class
            && !html::valid_class(o.class_name.trim())
        {
            return Err(ToolError::InvalidOptions(
                "클래스명은 영문자·숫자·-·_만 쓸 수 있고, 영문자나 _로 시작해야 합니다.".into(),
            ));
        }
        let path = input
            .first()
            .ok_or_else(|| ToolError::Failed("이미지를 넣어 주세요.".into()))?;
        // SR-only HTML의 <img src>: 파일 이름
        let src = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let a = analyze(ctx, path)?;
        Ok(ToolOutput::Blocks(blocks(&a, &o, &src)))
    }
}
