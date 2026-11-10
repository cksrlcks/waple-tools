use std::path::{Path, PathBuf};

use localtools_core::ToolError;

use crate::Fmt;

/// `out_dir/<입력 stem>.<포맷 확장자>`에 쓰고, 엔진에 돌려줄 상대 이름을 반환한다.
pub fn write_output(
    out_dir: &Path,
    input: &Path,
    fmt: Fmt,
    bytes: &[u8],
) -> Result<PathBuf, ToolError> {
    let stem = input
        .file_stem()
        .ok_or_else(|| ToolError::Failed("파일 이름이 없습니다.".into()))?;
    let name = PathBuf::from(format!("{}.{}", stem.to_string_lossy(), fmt.ext()));
    std::fs::write(out_dir.join(&name), bytes)?;
    Ok(name)
}
