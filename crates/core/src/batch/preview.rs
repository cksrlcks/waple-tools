//! 미리보기: 파일 하나에 도구를 임시 폴더로 실행한다. 원본·작업 기록은 건드리지 않는다.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::run::{PANIC_MESSAGE, inside};
use crate::{JobContext, Tool, ToolError, ToolOutput};

/// 결과 파일들의 절대 경로와, 드롭되면 지워지는 임시 폴더를 돌려준다. 오류는 사용자에게 보일 문구.
pub fn preview_one(
    tool: &dyn Tool,
    input: &Path,
    opts: &Value,
) -> Result<(tempfile::TempDir, Vec<PathBuf>), String> {
    let tmp = tempfile::Builder::new()
        .prefix("waple-tools-preview-")
        .tempdir()
        .map_err(|e| ToolError::from(e).to_string())?;
    let ctx = JobContext::new(|_| {}, Default::default());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tool.run(&ctx, &[input.to_path_buf()], tmp.path(), opts)
    }));
    let names = match outcome {
        Err(_) => return Err(PANIC_MESSAGE.into()),
        Ok(Err(e)) => return Err(e.to_string()),
        Ok(Ok(ToolOutput::Blocks(_))) => return Ok((tmp, vec![])),
        Ok(Ok(ToolOutput::Files(names))) => names,
    };
    if !names
        .iter()
        .all(|n| inside(n) && tmp.path().join(n).is_file())
    {
        return Err("도구가 잘못된 결과 경로를 돌려주었습니다.".into());
    }
    let files = names.iter().map(|n| tmp.path().join(n)).collect();
    Ok((tmp, files))
}
