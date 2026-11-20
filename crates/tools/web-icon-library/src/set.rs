//! 앱이 받아 둔 iconify 파일(`<dir>/<id>/<version>/`)을 `IconSet`으로 읽는다. 읽기 전용, 결과는 메모리에 캐시.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::sanitize::sanitize_body;

const BROKEN: &str = "받은 파일이 손상되었습니다. 다시 받아 주세요.";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub name: String,
    pub body: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IconSet {
    pub id: String,
    pub name: String,
    pub version: String,
    pub palette: bool,
    pub width: f64,
    pub height: f64,
    pub suffixes: BTreeMap<String, String>,
    pub icons: Vec<Icon>,
}

#[derive(Deserialize)]
struct RawIcons {
    #[serde(default)]
    icons: BTreeMap<String, RawIcon>,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Deserialize)]
struct RawIcon {
    body: String,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Deserialize)]
struct RawInfo {
    name: String,
    #[serde(default)]
    palette: bool,
}

#[derive(Deserialize)]
struct RawMeta {
    #[serde(default)]
    suffixes: BTreeMap<String, String>,
}

/// 세트·아이콘 이름 규칙: 소문자·숫자를 `-`로 이은 것 (`^[a-z0-9]+(-[a-z0-9]+)*$`).
pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// 폴더 이름으로 안전한 버전인지. 앱의 `store::safe_name`과 같은 규칙 (영숫자와 `. _ -`, `.`로 시작 금지).
fn safe_version(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('.')
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn build(
    id: &str,
    version: &str,
    icons_json: &[u8],
    info_json: &[u8],
    meta_json: Option<&[u8]>,
) -> Result<IconSet, String> {
    let bad = |_| BROKEN.to_string();
    let raw: RawIcons = serde_json::from_slice(icons_json).map_err(bad)?;
    let info: RawInfo = serde_json::from_slice(info_json).map_err(bad)?;
    // 이름 목록은 선택 파일이라 없거나 깨져도 세트는 만든다.
    let suffixes = meta_json
        .and_then(|b| serde_json::from_slice::<RawMeta>(b).ok())
        .map(|m| m.suffixes)
        .unwrap_or_default();
    // BTreeMap이라 이름순이다. 이름은 화면에서 id 치환 문자열에 들어가므로 이름 규칙이 아니면 빼고,
    // 정리에 실패했거나 비어 버린 아이콘도 뺀다.
    let icons = raw
        .icons
        .into_iter()
        .filter(|(name, _)| valid_name(name))
        .filter_map(|(name, i)| {
            Some(Icon {
                name,
                body: sanitize_body(&i.body).filter(|b| !b.trim().is_empty())?,
                width: i.width,
                height: i.height,
            })
        })
        .collect();
    Ok(IconSet {
        id: id.to_string(),
        name: info.name,
        version: version.to_string(),
        palette: info.palette,
        width: raw.width.unwrap_or(16.0),
        height: raw.height.unwrap_or(16.0),
        suffixes,
        icons,
    })
}

type Cache = Mutex<HashMap<(String, String), Arc<IconSet>>>;

/// `<dir>/<id>/<version>/{icons,info,metadata}.json`을 읽는다. 같은 `(id, version)`은 메모리에서 돌려준다.
pub fn load(dir: &Path, id: &str, version: &str) -> Result<Arc<IconSet>, String> {
    if !valid_name(id) || !safe_version(version) {
        return Err("리소스 이름이 올바르지 않습니다.".into());
    }
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (id.to_string(), version.to_string());
    if let Some(hit) = cache.lock().unwrap().get(&key) {
        return Ok(hit.clone());
    }
    let base = dir.join(id).join(version);
    let read = |n: &str| std::fs::read(base.join(n));
    let icons = read("icons.json").map_err(|_| "아이콘 세트 파일을 읽을 수 없습니다.")?;
    let info = read("info.json").map_err(|_| "아이콘 세트 정보를 읽을 수 없습니다.")?;
    let set = Arc::new(build(
        id,
        version,
        &icons,
        &info,
        read("metadata.json").ok().as_deref(),
    )?);
    cache.lock().unwrap().insert(key, set.clone());
    Ok(set)
}
