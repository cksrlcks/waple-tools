//! PDF 도구 공용: 열기(암호·손상 판별) · 페이지 범위 해석 · 저장 · 상속 속성 내리기

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
#[cfg(test)]
mod tests;

use std::io::Write;
use std::path::Path;

use lopdf::{Document, Object, ObjectId, SaveOptions};
use waple_tools_core::ToolError;

pub use lopdf;

/// PDF 확장자 (도구 meta.extensions)
pub const PDF_EXTS: &[&str] = &["pdf"];

pub fn unreadable() -> ToolError {
    ToolError::Failed("PDF를 읽을 수 없습니다 (손상되었거나 PDF가 아닙니다).".into())
}

pub fn encrypted() -> ToolError {
    ToolError::Failed("암호가 걸린 PDF는 처리할 수 없습니다.".into())
}

/// 파일 앞부분에 `%PDF-`가 있는가 (앞에 쓰레기 바이트가 붙은 파일도 있어 1KB 안을 본다)
pub fn looks_like_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|w| w == b"%PDF-")
}

/// 트레일러에 `/Encrypt`가 있는가. 소유자 암호만 걸린 PDF도 암호 PDF로 본다.
// ponytail: 바이트 검색이라 압축 안 된 내용에 "/Encrypt" 글자가 있으면 암호 PDF로 오판한다. 생기면 lopdf 트레일러로 확인
pub fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.windows(8).any(|w| w == b"/Encrypt")
}

/// 바이트를 열어 문서로. 암호·손상·빈 문서는 한국어 오류.
pub fn open_bytes(bytes: &[u8]) -> Result<Document, ToolError> {
    if !looks_like_pdf(bytes) {
        return Err(unreadable());
    }
    if is_encrypted(bytes) {
        return Err(encrypted());
    }
    let doc = Document::load_mem(bytes).map_err(|_| unreadable())?;
    if doc.was_encrypted() || doc.is_encrypted() {
        return Err(encrypted());
    }
    if doc.get_pages().is_empty() {
        return Err(ToolError::Failed("페이지가 없는 PDF입니다.".into()));
    }
    Ok(doc)
}

pub fn open(path: &Path) -> Result<Document, ToolError> {
    open_bytes(&std::fs::read(path)?)
}

/// 페이지 범위 `1-3,5,8-` → 1부터 시작하는 (처음, 끝) 목록. 빈 칸이면 전체 한 덩어리.
pub fn parse_ranges(text: &str, pages: u32) -> Result<Vec<(u32, u32)>, ToolError> {
    let bad = |m: String| ToolError::InvalidOptions(m);
    let text = text.trim();
    if text.is_empty() {
        return Ok(vec![(1, pages)]);
    }
    let num = |s: &str| -> Result<u32, ToolError> {
        s.trim().parse::<u32>().map_err(|_| {
            bad(format!(
                "페이지 범위를 읽을 수 없습니다: \"{text}\" (예: 1-3,5,8-)"
            ))
        })
    };
    let mut out = vec![];
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (
                if a.trim().is_empty() { 1 } else { num(a)? },
                if b.trim().is_empty() { pages } else { num(b)? },
            ),
            None => {
                let n = num(part)?;
                (n, n)
            }
        };
        if a == 0 || a > b {
            return Err(bad(format!("페이지 범위가 올바르지 않습니다: {part}")));
        }
        if b > pages {
            return Err(bad(format!(
                "페이지 범위 {part}가 문서 페이지 수({pages})를 넘습니다."
            )));
        }
        out.push((a, b));
    }
    if out.is_empty() {
        return Err(bad("페이지 범위를 입력하세요.".into()));
    }
    Ok(out)
}

/// 범위 목록 → 페이지 번호 (중복은 처음 것만, 순서 유지)
pub fn pages_in(ranges: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = std::collections::BTreeSet::new();
    ranges
        .iter()
        .flat_map(|&(a, b)| a..=b)
        .filter(|p| seen.insert(*p))
        .collect()
}

/// 페이지 트리 위쪽에서 물려받는 속성을 각 페이지에 직접 적는다 (페이지를 다른 트리로 옮기기 전에)
pub fn push_down_inherited(doc: &mut Document) {
    const KEYS: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for page in pages {
        let mut found: Vec<(&[u8], Object)> = vec![];
        let Ok(dict) = doc.get_dictionary(page) else {
            continue;
        };
        let mut missing: Vec<&[u8]> = KEYS.into_iter().filter(|k| !dict.has(k)).collect();
        let mut parent = dict.get(b"Parent").and_then(Object::as_reference).ok();
        // 순환 참조 대비 깊이 제한
        for _ in 0..64 {
            let Some(id) = parent else { break };
            if missing.is_empty() {
                break;
            }
            let Ok(node) = doc.get_dictionary(id) else {
                break;
            };
            missing.retain(|k| match node.get(k) {
                Ok(v) => {
                    found.push((k, v.clone()));
                    false
                }
                Err(_) => true,
            });
            parent = node.get(b"Parent").and_then(Object::as_reference).ok();
        }
        if let Ok(dict) = doc.get_dictionary_mut(page) {
            for (k, v) in found {
                dict.set(k, v);
            }
        }
    }
}

/// 페이지를 붙잡는 문서 단위 항목(북마크·이름 붙은 목적지·태그 구조)을 뺀다.
/// 나누기·합치기 결과에서 지운 페이지가 이것 때문에 파일에 남지 않게 한다.
pub fn drop_page_refs(doc: &mut Document) {
    if let Ok(catalog) = doc.catalog_mut() {
        for key in [&b"Outlines"[..], b"StructTreeRoot", b"Dests", b"OpenAction"] {
            catalog.remove(key);
        }
    }
    let names = doc
        .catalog()
        .ok()
        .and_then(|c| c.get(b"Names").ok())
        .and_then(|n| n.as_reference().ok());
    match names {
        Some(id) => {
            if let Ok(names) = doc.get_dictionary_mut(id) {
                names.remove(b"Dests");
            }
        }
        None => {
            if let Ok(Object::Dictionary(names)) =
                doc.catalog_mut().and_then(|c| c.get_mut(b"Names"))
            {
                names.remove(b"Dests");
            }
        }
    }
}

/// 스트림을 압축하고 객체 스트림·xref 스트림(PDF 1.5)으로 저장한다
pub fn save(doc: &mut Document, path: &Path) -> Result<(), ToolError> {
    doc.compress();
    if doc.version.as_str() < "1.5" {
        doc.version = "1.5".into();
    }
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let opts = SaveOptions::builder()
        .use_object_streams(true)
        .use_xref_streams(true)
        .build();
    doc.save_with_options(&mut file, opts)
        .map_err(|e| ToolError::Failed(format!("PDF를 저장할 수 없습니다: {e}")))?;
    file.flush()?;
    Ok(())
}

/// 옵션 JSON → 구조체. 빠진 필드는 구조체의 `#[serde(default)]`로 채운다. (image-common과 같은 규칙)
pub fn parse_options<T: serde::de::DeserializeOwned>(
    opts: &serde_json::Value,
) -> Result<T, ToolError> {
    let value = if opts.is_null() {
        serde_json::json!({})
    } else {
        opts.clone()
    };
    serde_json::from_value(value).map_err(|e| ToolError::InvalidOptions(e.to_string()))
}

pub fn schema_of<T: schemars::JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// 결과 파일 `<out_dir>/<rel>`을 만들 폴더까지 준비해 경로를 돌려준다
pub fn out_path(out_dir: &Path, rel: &Path) -> Result<std::path::PathBuf, ToolError> {
    let path = out_dir.join(rel);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(path)
}

/// 파일 이름 줄기 (확장자 뺀 이름)
pub fn stem(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
