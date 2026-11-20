//! 웹 아이콘 라이브러리 추가 도구 (Interactive, 입력 파일 없음). 무료 Iconify 세트 목록(스냅샷),
//! 받아 둔 세트의 아이콘 격자, 고른 아이콘의 PNG·SVG·JSX·Data URI·클래스.
//! 디스크에는 쓰지 않고(읽기만), 네트워크도 쓰지 않는다. 받기는 앱의 공용 리소스 받기가 맡는다.

pub mod blocks;
pub mod collections;
pub mod png;
pub mod render;
pub mod sanitize;
pub mod set;
pub mod styles;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

// `set`·`style`·`query`·`selected`는 결과 블록(목록·격자)이 바꾸는 숨김 옵션.
// `__resources`는 스키마에 없고 앱이 넣는다 (없으면 받은 세트 없음).
#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-input" = "none",
    "x-resources" = {
        "source": "npm",
        "package": "@iconify-json/{id}",
        "files": ["icons.json", "info.json", "metadata.json"]
    },
    "x-stage" = "list-detail",
    "x-stage-plain" = true,
    "x-panel-when" = "selected",
    "x-panel-position" = "bottom",
    "x-panel-blocks" = ["image", "actions"]
))]
pub struct Options {
    #[schemars(extend("x-hidden" = true))]
    pub set: String,
    #[schemars(extend("x-hidden" = true))]
    pub style: String,
    #[schemars(extend("x-hidden" = true))]
    pub query: String,
    #[schemars(extend("x-hidden" = true))]
    pub selected: String,
    /// 크기(px)
    ///
    /// 아이콘 높이입니다. 가로는 아이콘 비율에 맞춥니다. PNG도 이 크기로 만듭니다.
    #[schemars(range(min = 8, max = 512))]
    pub size: u32,
    /// 두께
    ///
    /// 선 아이콘의 stroke-width를 바꿉니다. [원래대로]를 누르면 원래 두께로 돌아갑니다.
    #[schemars(range(min = 0.5, max = 3), extend("x-step" = 0.25))]
    pub stroke: Option<f64>,
    /// 색
    ///
    /// 컬러 아이콘 세트는 색을 바꾸지 않습니다.
    #[schemars(extend("format" = "color", "x-swatches" = ["#000000", "#6B7280", "#2563EB", "#DC2626"]))]
    pub color: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            set: String::new(),
            style: String::new(),
            query: String::new(),
            selected: String::new(),
            size: 24,
            stroke: None,
            color: "#000000".into(),
        }
    }
}

struct WebIconLibrary;

pub fn tool() -> Box<dyn Tool> {
    Box::new(WebIconLibrary)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for WebIconLibrary {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "web.icon_library".into(),
            name: "아이콘 라이브러리".into(),
            category: "web".into(),
            description: "무료 아이콘 세트를 받아 검색하고 SVG·PNG·클래스로 복사합니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(Options).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: Options = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        // 형식이 틀리면 받은 세트 없음으로 본다
        let res: Option<blocks::Resources> = opts
            .get("__resources")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        ctx.check_cancel()?;
        Ok(ToolOutput::Blocks(blocks::build(&o, res.as_ref())))
    }
}

#[cfg(test)]
mod tests;
