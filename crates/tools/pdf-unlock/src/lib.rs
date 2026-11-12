//! PDF 비밀번호 해제 (추가 설치, PerFile): 여는 암호나 권한 암호를 알면 암호 없는 PDF로 저장한다

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_pdf_common::lopdf::encryption::PasswordAlgorithm;
use waple_tools_pdf_common::lopdf::{Document, Error, LoadOptions};
use waple_tools_pdf_common::{
    PDF_EXTS, looks_like_pdf, out_path, parse_options, save, schema_of, unreadable,
};

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
// 암호를 푼 결과가 원본보다 커도 버리지 않는다 ("커지면 원본 유지"면 암호 PDF가 그대로 남는다)
#[schemars(extend(
    "x-preview" = "none",
    "x-allow-larger" = true,
    "x-files-only" = true,
    "x-keep-original" = true,
    "x-input-grid" = true,
    "x-single-input" = true,
))]
struct UnlockOptions {
    /// 비밀번호
    ///
    /// 여는 암호나 권한 암호 중 하나를 넣습니다. 입력한 비밀번호는 저장하지 않습니다.
    #[schemars(extend("format" = "password"))]
    password: String,
}

struct PdfUnlock;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfUnlock)
}

waple_tools_addon_abi::export_tool!(tool);

fn wrong_password() -> ToolError {
    ToolError::Failed("비밀번호가 맞지 않습니다.".into())
}

/// 암호를 풀어 연 문서. 입력한 비밀번호가 여는 암호나 권한 암호와 맞아야 한다.
fn unlock(bytes: &[u8], password: &str) -> Result<Document, ToolError> {
    if !looks_like_pdf(bytes) {
        return Err(unreadable());
    }
    if password.is_empty() {
        return Err(ToolError::InvalidOptions("비밀번호를 입력하세요.".into()));
    }
    let mut doc = Document::load_mem_with_options(bytes, LoadOptions::with_password(password))
        .map_err(|e| match e {
            Error::InvalidPassword => wrong_password(),
            _ => unreadable(),
        })?;
    // lopdf는 빈 암호를 먼저 시도해서, 권한 암호만 걸린 PDF는 아무 비밀번호로나 열린다.
    // 읽을 때 지운 /Encrypt를 잠시 되돌려 입력한 비밀번호를 직접 확인한다.
    let state = doc
        .encryption_state
        .as_ref()
        .ok_or_else(|| ToolError::Failed("암호가 걸리지 않은 PDF입니다.".into()))?;
    let dict = state.encode().map_err(|_| unreadable())?;
    let id = doc.add_object(dict);
    doc.trailer.set("Encrypt", id);
    // RC4·AES-128(R4 이하)은 비밀번호를 PDFDocEncoding으로 바꾸며 한글을 버린다.
    // 다 버려져 빈 값이 되면 빈 여는 암호와 맞아 버리므로 틀린 비밀번호로 본다.
    let ok = PasswordAlgorithm::try_from(&doc)
        .ok()
        .and_then(|a| a.sanitize_password(password).ok())
        .is_some_and(|raw| !raw.is_empty() && doc.authenticate_raw_password(&raw).is_ok());
    doc.trailer.remove(b"Encrypt");
    doc.objects.remove(&id);
    if !ok {
        return Err(wrong_password());
    }
    if doc.get_pages().is_empty() {
        return Err(ToolError::Failed("페이지가 없는 PDF입니다.".into()));
    }
    Ok(doc)
}

impl Tool for PdfUnlock {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.unlock".into(),
            name: "비밀번호 해제".into(),
            category: "pdf".into(),
            description: "PDF의 암호를 풀어 저장합니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<UnlockOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: UnlockOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("PDF를 넣어 주세요.".into()))?;
        let mut doc = unlock(&std::fs::read(src)?, &o.password)?;
        ctx.check_cancel()?;
        let rel = PathBuf::from(src.file_name().unwrap_or_default());
        save(&mut doc, &out_path(out_dir, &rel)?)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
