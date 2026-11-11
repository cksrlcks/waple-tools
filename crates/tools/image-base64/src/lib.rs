//! 이미지 → Base64 (추가 설치, Interactive): Data URI · CSS background · `<img>` 태그 코드를 화면에 보여 준다.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

/// 이보다 크면 경고 (코드에 넣으면 HTML·CSS가 무거워진다)
const WARN_BYTES: u64 = 32 * 1024;
/// 이보다 크면 만들지 않는다 (화면이 수십 MB 문자열을 그리지 못한다)
const MAX_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Format {
    /// Data URI
    #[default]
    DataUri,
    /// CSS background
    Css,
    /// <img> 태그
    ImgTag,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct Base64Options {
    /// 출력 형식
    format: Format,
}

struct ImageBase64;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageBase64)
}

waple_tools_addon_abi::export_tool!(tool);

/// 파일 앞부분으로 MIME을 고른다 (확장자는 믿지 않는다)
fn mime(bytes: &[u8]) -> Option<&'static str> {
    let starts = |m: &[u8]| bytes.starts_with(m);
    if starts(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if starts(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if starts(b"GIF87a") || starts(b"GIF89a") {
        Some("image/gif")
    } else if starts(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if starts(b"BM") {
        Some("image/bmp")
    } else if starts(b"II*\0") || starts(b"MM\0*") {
        Some("image/tiff")
    } else if starts(b"\0\0\x01\0") {
        Some("image/x-icon")
    } else if is_svg(bytes) {
        Some("image/svg+xml")
    } else {
        None
    }
}

fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start_matches('\u{feff}').trim_start();
    (t.starts_with("<?xml") || t.starts_with("<svg") || t.starts_with("<!--")) && t.contains("<svg")
}

fn kb(n: u64) -> f64 {
    (n as f64 / 1024.0 * 10.0).round() / 10.0
}

fn encode(path: &Path, format: Format) -> Result<Vec<Block>, ToolError> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_BYTES {
        return Err(ToolError::Failed(
            "파일이 너무 큽니다. 10MB 이하 이미지만 바꿀 수 있습니다.".into(),
        ));
    }
    let bytes = std::fs::read(path)?;
    let mime =
        mime(&bytes).ok_or_else(|| ToolError::Failed("지원하지 않는 이미지 형식입니다.".into()))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let uri = format!("data:{mime};base64,{b64}");
    let (lang, code) = match format {
        Format::DataUri => ("text", uri),
        Format::Css => ("css", format!("background-image: url(\"{uri}\");")),
        Format::ImgTag => ("html", format!("<img src=\"{uri}\" alt=\"\">")),
    };
    let mut blocks = vec![];
    if size > WARN_BYTES {
        blocks.push(Block::Text {
            text: format!(
                "용량이 큰 이미지({} KB)입니다. 코드에 직접 넣으면 HTML·CSS가 그만큼 무거워지고 캐시도 따로 되지 않습니다. 작은 아이콘·이미지에만 쓰는 것이 좋습니다.",
                kb(size)
            ),
        });
    }
    blocks.push(Block::Number {
        label: "원본 용량".into(),
        value: kb(size),
        unit: Some("KB".into()),
    });
    blocks.push(Block::Number {
        label: "코드 길이".into(),
        value: kb(code.len() as u64),
        unit: Some("KB".into()),
    });
    blocks.push(Block::Code {
        lang: lang.into(),
        code,
    });
    Ok(blocks)
}

impl Tool for ImageBase64 {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.base64".into(),
            name: "이미지 → Base64".into(),
            category: "image".into(),
            description: "이미지를 Data URI 등 Base64 코드로 바꿉니다.".into(),
            extensions: [
                "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico", "svg",
            ]
            .map(String::from)
            .to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(Base64Options).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: Base64Options = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        ctx.check_cancel()?;
        let path = input
            .first()
            .ok_or_else(|| ToolError::Failed("이미지를 넣어 주세요.".into()))?;
        encode(path, opts.format).map(ToolOutput::Blocks)
    }
}

#[cfg(test)]
mod tests;
