//! 도구 크레이트 공용: 옵션 해석, 스키마, 입력 파일마다 실행

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use waple_tools_core::{JobContext, ToolError, ToolOutput};

/// 옵션 JSON → 구조체. 빠진 필드는 구조체의 `#[serde(default)]`로 채운다.
pub fn parse_options<T: DeserializeOwned>(opts: &serde_json::Value) -> Result<T, ToolError> {
    let value = if opts.is_null() {
        serde_json::json!({})
    } else {
        opts.clone()
    };
    serde_json::from_value(value).map_err(|e| ToolError::InvalidOptions(e.to_string()))
}

pub fn schema_of<T: JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// 입력마다 `f(입력)`을 실행해 결과 이름을 모은다. 파일 사이마다 취소를 확인한다.
pub fn run_each(
    ctx: &JobContext,
    input: &[PathBuf],
    mut f: impl FnMut(&Path) -> Result<PathBuf, ToolError>,
) -> Result<ToolOutput, ToolError> {
    let mut names = Vec::with_capacity(input.len());
    for (i, p) in input.iter().enumerate() {
        ctx.check_cancel()?;
        names.push(f(p)?);
        ctx.progress((i + 1) as f32 / input.len().max(1) as f32);
    }
    Ok(ToolOutput::Files(names))
}
