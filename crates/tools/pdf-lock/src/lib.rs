//! PDF 비밀번호 걸기 (추가 설치, PerFile): 여는 암호를 AES-256으로 건다

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_pdf_common::lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
use waple_tools_pdf_common::lopdf::{
    Document, EncryptionState, EncryptionVersion, Object, Permissions, StringFormat,
};
use waple_tools_pdf_common::{
    PDF_EXTS, is_encrypted, looks_like_pdf, open_bytes, out_path, parse_options, schema_of,
};

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
// 암호를 건 결과는 원본보다 커도 쓴다. 파일 하나만, 넣을 때 이미 암호 PDF인지 확인한다
#[schemars(extend(
    "x-preview" = "none",
    "x-allow-larger" = true,
    "x-files-only" = true,
    "x-keep-original" = true,
    "x-single-input" = true,
    "x-input-grid" = true,
    "x-check-input" = true,
))]
struct LockOptions {
    /// 비밀번호
    ///
    /// PDF를 열 때 넣을 비밀번호입니다. AES-256으로 암호화하며 한글도 쓸 수 있습니다. 입력한 비밀번호는 저장하지 않습니다.
    #[schemars(extend("format" = "password"))]
    password: String,
    /// 비밀번호 확인
    #[schemars(extend("format" = "password"))]
    password_confirm: String,
}

struct PdfLock;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfLock)
}

waple_tools_addon_abi::export_tool!(tool);

fn random<const N: usize>() -> Result<[u8; N], ToolError> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf)
        .map_err(|_| ToolError::Failed("암호화 키를 만들 수 없습니다.".into()))?;
    Ok(buf)
}

/// 암호가 없는 PDF만 연다. 이미 암호(여는 암호나 권한 암호)가 걸렸으면 그렇다고 알린다
fn open_unlocked(src: &Path) -> Result<Document, ToolError> {
    let bytes = std::fs::read(src)?;
    if looks_like_pdf(&bytes) && is_encrypted(&bytes) {
        return Err(ToolError::Failed("이미 비밀번호가 걸린 PDF입니다.".into()));
    }
    open_bytes(&bytes)
}

/// 문서에 AES-256 여는 암호를 건다. 권한 제한은 두지 않는다 (권한 암호 = 여는 암호)
fn lock(doc: &mut Document, password: &str) -> Result<(), ToolError> {
    // 암호 PDF는 /ID가 있어야 한다 (PDF 규격)
    if doc.trailer.get(b"ID").is_err() {
        let id = Object::String(random::<16>()?.to_vec(), StringFormat::Hexadecimal);
        doc.trailer.set("ID", vec![id.clone(), id]);
    }
    let key = random::<32>()?;
    let filter: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);
    let state = EncryptionState::try_from(EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), filter)]),
        file_encryption_key: &key,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: password,
        user_password: password,
        permissions: Permissions::all(),
    })
    .map_err(|e| ToolError::Failed(format!("암호를 걸 수 없습니다: {e}")))?;
    doc.encrypt(&state)
        .map_err(|e| ToolError::Failed(format!("암호를 걸 수 없습니다: {e}")))
}

impl Tool for PdfLock {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.lock".into(),
            name: "비밀번호 걸기".into(),
            category: "pdf".into(),
            description: "PDF에 비밀번호를 걸어 저장합니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<LockOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("PDF를 넣어 주세요.".into()))?;
        // 넣을 때 확인 (`x-check-input`): 열 수 있는 PDF면 빈 결과, 아니면 그 오류
        if opts["x-check-input"] == true {
            open_unlocked(src)?;
            return Ok(ToolOutput::Blocks(vec![]));
        }
        let o: LockOptions = parse_options(opts)?;
        if o.password.is_empty() {
            return Err(ToolError::InvalidOptions("비밀번호를 입력하세요.".into()));
        }
        if o.password != o.password_confirm {
            return Err(ToolError::InvalidOptions(
                "비밀번호 확인이 맞지 않습니다.".into(),
            ));
        }
        let mut doc = open_unlocked(src)?;
        ctx.check_cancel()?;
        // 압축은 암호화 전에. 암호화 뒤에 압축하거나 객체 스트림으로 묶으면 풀 수 없는 PDF가 된다
        // (그래서 pdf-common의 save를 쓰지 않는다)
        doc.compress();
        lock(&mut doc, &o.password)?;
        let rel = PathBuf::from(src.file_name().unwrap_or_default());
        let mut file = std::io::BufWriter::new(std::fs::File::create(out_path(out_dir, &rel)?)?);
        doc.save_to(&mut file)
            .map_err(|e| ToolError::Failed(format!("PDF를 저장할 수 없습니다: {e}")))?;
        file.flush()?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
