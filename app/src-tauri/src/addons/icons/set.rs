//! 받은 iconify 파일을 `IconSet`(set.json · 프론트 공통)으로 바꾸고, 무료 세트 목록을 거른다.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::npm::valid_prefix;
use super::sanitize::sanitize_body;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub title: String,
    pub spdx: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub name: String,
    pub body: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IconSet {
    pub prefix: String,
    pub name: String,
    pub version: String,
    pub license: License,
    pub palette: bool,
    pub width: u32,
    pub height: u32,
    pub suffixes: BTreeMap<String, String>,
    pub icons: Vec<Icon>,
}

#[derive(Deserialize)]
struct RawIcons {
    #[serde(default)]
    icons: BTreeMap<String, RawIcon>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct RawIcon {
    body: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct RawInfo {
    name: String,
    license: RawLicense,
    #[serde(default)]
    palette: bool,
}

#[derive(Deserialize)]
struct RawLicense {
    title: String,
    spdx: Option<String>,
}

#[derive(Deserialize)]
struct RawMeta {
    #[serde(default)]
    suffixes: BTreeMap<String, String>,
}

pub fn build_set(
    prefix: &str,
    version: &str,
    files: &HashMap<String, Vec<u8>>,
) -> Result<IconSet, String> {
    let bad = |_| "받은 파일이 손상되었습니다. 다시 받아 주세요.".to_string();
    let icons_json = files
        .get("icons.json")
        .ok_or("아이콘 세트 파일을 읽을 수 없습니다.")?;
    let info_json = files
        .get("info.json")
        .ok_or("아이콘 세트 정보를 읽을 수 없습니다.")?;
    let raw: RawIcons = serde_json::from_slice(icons_json).map_err(bad)?;
    let info: RawInfo = serde_json::from_slice(info_json).map_err(bad)?;
    // 이름 목록은 선택 파일이라 없거나 깨져도 세트는 만든다.
    let suffixes = files
        .get("metadata.json")
        .and_then(|b| serde_json::from_slice::<RawMeta>(b).ok())
        .map(|m| m.suffixes)
        .unwrap_or_default();
    // BTreeMap이라 이름순이다. 정리에 실패한 아이콘은 뺀다.
    let icons = raw
        .icons
        .into_iter()
        .filter_map(|(name, i)| {
            Some(Icon {
                name,
                body: sanitize_body(&i.body)?,
                width: i.width,
                height: i.height,
            })
        })
        .collect();
    Ok(IconSet {
        prefix: prefix.to_string(),
        name: info.name,
        version: version.to_string(),
        license: License {
            title: info.license.title,
            spdx: info.license.spdx,
        },
        palette: info.palette,
        width: raw.width.unwrap_or(16),
        height: raw.height.unwrap_or(16),
        suffixes,
        icons,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub prefix: String,
    pub name: String,
    pub total: u32,
    pub category: Option<String>,
    /// spdx 식별자
    pub license: String,
    pub palette: bool,
}

#[derive(Deserialize)]
struct RawCollection {
    name: String,
    #[serde(default)]
    total: u32,
    category: Option<String>,
    license: Option<RawLicense>,
    #[serde(default)]
    palette: bool,
    #[serde(default)]
    hidden: bool,
}

const FREE: &[&str] = &["MIT", "Apache-2.0", "ISC", "CC0-1.0"];

/// 무료 라이선스이고 숨김이 아닌 세트만, 이름순으로.
pub fn free_collections(json: &[u8]) -> Result<Vec<Collection>, String> {
    let raw: BTreeMap<String, RawCollection> = serde_json::from_slice(json)
        .map_err(|_| "받은 파일이 손상되었습니다. 다시 받아 주세요.".to_string())?;
    let mut list: Vec<Collection> = raw
        .into_iter()
        .filter(|(p, c)| valid_prefix(p) && !c.hidden)
        .filter_map(|(prefix, c)| {
            let spdx = c.license?.spdx.filter(|s| FREE.contains(&s.as_str()))?;
            Some(Collection {
                prefix,
                name: c.name,
                total: c.total,
                category: c.category,
                license: spdx,
                palette: c.palette,
            })
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}
