//! 명도대비 렌즈 (추가 설치, AI, Interactive, 입력 파일 없음): 떠 있는 렌즈 안 글자·아이콘의 대비가 기준을 넘는지 표시.
//! 글자 영역은 PP-OCRv5 mobile det(DLL 안)으로 찾고, 대비는 픽셀로 계산한다. 아이콘(작고 색이 한두 가지)은 픽셀 덩어리로 찾아 3:1. 검사는 띄울 때·[검사]·옮긴 뒤에만.

mod analyze;
mod lens;
mod ocr;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Action {
    /// 렌즈 띄우기
    #[default]
    Show,
}

/// 옵션은 실행 버튼 하나뿐: 기준은 렌즈 손잡이에서 고른다 (사용자 결정)
#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none"))]
struct Options {
    /// 렌즈
    #[schemars(extend("x-run-buttons" = true))]
    #[allow(dead_code)] // 실행 버튼 자리: 값은 하나뿐
    action: Action,
}

/// 고른 기준 (렌즈 손잡이 토글). 처음엔 둘 다 켬
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Crit {
    /// 글자 4.5:1
    pub text: bool,
    /// 글자 3:1
    pub large: bool,
}

impl Crit {
    pub const START: Self = Self {
        text: true,
        large: true,
    };

    /// 4.5:1을 골랐으면 4.5, 아니면 3
    pub fn need(self) -> f32 {
        if self.text { 4.5 } else { 3.0 }
    }

    /// 4.5:1을 켜고 끈다. 둘 다 꺼지면 그대로
    pub fn toggle_text(self) -> Self {
        Self {
            text: !self.text,
            ..self
        }
        .or(self)
    }

    /// 3:1을 켜고 끈다. 둘 다 꺼지면 그대로
    pub fn toggle_large(self) -> Self {
        Self {
            large: !self.large,
            ..self
        }
        .or(self)
    }

    fn or(self, prev: Self) -> Self {
        if self.text || self.large { self } else { prev }
    }
}

/// 대비는 반올림하지 않고 버린다: 4.48이 4.5로 보이면 통과로 오해한다
pub(crate) fn fmt_ratio(r: f32) -> String {
    format!("{:.1}:1", (r * 10.0).floor() / 10.0)
}

/// 미달 상자에 붙이는 글 (어느 기준까지 충족하는지는 쓰지 않는다, 사용자 결정)
pub(crate) fn fail_label(ratio: f32) -> String {
    format!("{} 미달", fmt_ratio(ratio))
}

struct ContrastLens;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ContrastLens)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for ContrastLens {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "a11y.contrast_lens".into(),
            name: "명도대비 렌즈".into(),
            category: "a11y".into(),
            description: "떠 있는 렌즈 안 글자·아이콘의 명도대비가 기준을 넘는지 표시합니다."
                .into(),
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
        _ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        _opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        if !ocr::available() {
            return Err(ToolError::Failed(
                "이 빌드에는 AI 모델이 들어 있지 않습니다.".into(),
            ));
        }
        lens::show()?;
        // 안내 글 없음 (사용자 결정): 렌즈만 띄운다
        Ok(ToolOutput::Blocks(vec![]))
    }
}

#[cfg(test)]
mod tests;
