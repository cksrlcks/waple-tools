//! 폰트 변환 (추가 설치, PerFile): 고른 형식으로 바꾼다. WOFF2 ← TTF·OTF·WOFF, TTF·OTF ← WOFF·WOFF2. 입력 형식은 파일 내용으로 판단한다.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Target {
    /// WOFF2 (웹폰트)
    #[default]
    Woff2,
    /// TTF·OTF (원래 형식)
    Sfnt,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-preview" = "none",
    // WOFF2를 풀면 늘 커진다: "결과가 크면 원본 유지" 설정을 쓰지 않는다
    "x-allow-larger" = true,
    // 형식은 파일마다 고른다 (목록의 각 파일 옆). WOFF2 파일은 처음에 "TTF·OTF"로
    "x-per-file" = { "format": { "woff2": "sfnt" } }
))]
struct Woff2Options {
    /// 변환할 형식
    ///
    /// WOFF2: TTF·OTF·WOFF를 웹폰트로 압축합니다. TTF·OTF: WOFF·WOFF2를 풀어 원래 형식으로 되돌립니다 (TTF와 OTF끼리는 바꿀 수 없습니다).
    format: Target,
}

struct WebWoff2;

pub fn tool() -> Box<dyn Tool> {
    Box::new(WebWoff2)
}

waple_tools_addon_abi::export_tool!(tool);

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    TrueType,
    OpenType,
    Woff,
    Woff2,
}

/// 파일 앞 4바이트로 형식을 고른다 (확장자는 믿지 않는다)
fn kind(bytes: &[u8]) -> Result<Kind, ToolError> {
    match bytes.get(..4) {
        Some(b"wOF2") => Ok(Kind::Woff2),
        Some(b"wOFF") => Ok(Kind::Woff),
        Some(b"OTTO") => Ok(Kind::OpenType),
        Some([0, 1, 0, 0]) | Some(b"true") => Ok(Kind::TrueType),
        Some(b"ttcf") => Err(ToolError::Failed(
            "폰트 모음(TTC)은 지원하지 않습니다. 폰트 하나씩 된 TTF·OTF를 넣어 주세요.".into(),
        )),
        _ => Err(ToolError::Failed(
            "폰트 파일을 읽을 수 없습니다 (TTF·OTF·WOFF·WOFF2가 아닙니다).".into(),
        )),
    }
}

fn broken() -> ToolError {
    ToolError::Failed("폰트 파일이 손상되었거나 읽을 수 없습니다.".into())
}

/// WOFF·WOFF2 → sfnt(TTF·OTF). 이미 sfnt면 그대로
fn to_sfnt(bytes: &[u8], kind: &Kind) -> Result<Vec<u8>, ToolError> {
    match kind {
        Kind::Woff2 => wuff::decompress_woff2(bytes).map_err(|_| broken()),
        Kind::Woff => wuff::decompress_woff1(bytes).map_err(|_| broken()),
        Kind::TrueType | Kind::OpenType => Ok(bytes.to_vec()),
    }
}

/// 바꾼 내용과 결과 확장자
fn convert(bytes: &[u8], target: Target) -> Result<(Vec<u8>, &'static str), ToolError> {
    let kind = kind(bytes)?;
    match target {
        Target::Woff2 => {
            if kind == Kind::Woff2 {
                return Err(ToolError::Failed("이미 WOFF2 형식입니다.".into()));
            }
            let sfnt = to_sfnt(bytes, &kind)?;
            let out = ttf2woff2::encode(&sfnt, ttf2woff2::BrotliQuality::default())
                .map_err(|_| broken())?;
            Ok((out, "woff2"))
        }
        Target::Sfnt => {
            if matches!(kind, Kind::TrueType | Kind::OpenType) {
                return Err(ToolError::Failed("이미 TTF·OTF 형식입니다.".into()));
            }
            let out = to_sfnt(bytes, &kind)?;
            let ext = if out.starts_with(b"OTTO") {
                "otf"
            } else {
                "ttf"
            };
            Ok((out, ext))
        }
    }
}

impl Tool for WebWoff2 {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "web.woff2".into(),
            name: "폰트 변환 (WOFF2)".into(),
            category: "web".into(),
            description: "TTF·OTF를 WOFF2 웹폰트로, WOFF2를 TTF·OTF로 바꿉니다.".into(),
            extensions: ["ttf", "otf", "woff", "woff2"].map(String::from).to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(Woff2Options).to_value()
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
            .ok_or_else(|| ToolError::Failed("폰트 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let o: Woff2Options = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        let (out, ext) = convert(&std::fs::read(src)?, o.format)?;
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let rel = PathBuf::from(format!("{stem}.{ext}"));
        std::fs::write(out_dir.join(&rel), out)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
