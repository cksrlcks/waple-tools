//! 색맹 안경 (추가 설치, Interactive, 입력 파일 없음): 아래 화면을 색각이상 시뮬레이션으로 비추는 떠 있는 렌즈

mod lens;
mod matrix;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

use matrix::DEFAULT_MODE;

/// 순서는 `matrix::MODES`와 같다
#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum LensMode {
    /// 적색맹
    ///
    /// 빨강이 어둡게, 빨강·초록·갈색이 비슷하게 보임
    Protan,
    /// 녹색맹
    ///
    /// 빨강·초록이 누렇게 비슷해짐 (가장 흔함)
    #[default]
    Deutan,
    /// 청색맹
    ///
    /// 파랑·초록, 노랑·분홍이 헷갈림
    Tritan,
    /// 전색맹
    ///
    /// 색 없이 밝기만 보임
    Achroma,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none"))]
struct LensOptions {
    /// 보기 방식
    #[schemars(extend("x-run-buttons" = true))]
    mode: LensMode,
}

/// 옵션의 보기 방식 번호. 없거나 모르는 값이면 녹색맹
pub(crate) fn mode_index(opts: &serde_json::Value) -> usize {
    match serde_json::from_value::<LensOptions>(opts.clone()) {
        Ok(o) => o.mode as usize,
        Err(_) => DEFAULT_MODE,
    }
}

struct CvdLens;

pub fn tool() -> Box<dyn Tool> {
    Box::new(CvdLens)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for CvdLens {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "a11y.cvd_lens".into(),
            name: "색맹 안경".into(),
            category: "a11y".into(),
            description: "색각이상인 사람 눈에 보이는 화면을 떠 있는 렌즈로 봅니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(LensOptions).to_value()
    }

    fn run(
        &self,
        _ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        lens::show(mode_index(opts))?;
        Ok(ToolOutput::Blocks(vec![Block::Text {
            text: "렌즈를 띄웠습니다. 손잡이를 끌어 옮기고 Esc로 닫습니다.".into(),
        }]))
    }
}

#[cfg(test)]
mod tests;
