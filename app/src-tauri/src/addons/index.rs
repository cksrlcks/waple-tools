//! 원격 목록(`index.json`)과 설치 기록(`installed.json`) 형식.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Releases `addons` 태그의 `index.json`. 도구마다 ABI 버전별 최신판 하나씩.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    pub tools: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub version: String,
    pub abi: u32,
    pub url: String,
    pub size: u64,
    /// 소문자 16진수
    pub sha256: String,
    /// `tauri signer sign`이 만든 `.sig` 파일 내용 (base64)
    pub signature: String,
}

/// 이 앱이 설치할 판: 같은 ABI 중 가장 높은 버전
pub fn pick<'a>(index: &'a Index, id: &str, abi: u32) -> Option<&'a IndexEntry> {
    index
        .tools
        .iter()
        .filter(|e| e.id == id && e.abi == abi)
        .max_by(|a, b| cmp_version(&a.version, &b.version))
}

/// `1.10.0` > `1.9.2`. 숫자가 아닌 조각은 0으로 본다.
pub fn cmp_version(a: &str, b: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let (a, b) = (parse(a), parse(b));
    let n = a.len().max(b.len());
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    (0..n)
        .map(|i| at(&a, i).cmp(&at(&b, i)))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// `addons/installed.json`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Installed {
    pub tools: BTreeMap<String, InstalledTool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledTool {
    pub version: String,
    pub abi: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, version: &str, abi: u32) -> IndexEntry {
        IndexEntry {
            id: id.into(),
            version: version.into(),
            abi,
            url: String::new(),
            size: 0,
            sha256: String::new(),
            signature: String::new(),
        }
    }

    #[test]
    fn versions_compare_numerically() {
        assert_eq!(cmp_version("1.10.0", "1.9.2"), Ordering::Greater);
        assert_eq!(cmp_version("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(cmp_version("0.9.9", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn pick_takes_latest_for_own_abi() {
        let index = Index {
            tools: vec![
                entry("image.base64", "1.2.0", 1),
                entry("image.base64", "1.10.0", 1),
                entry("image.base64", "2.0.0", 2),
                entry("image.icon", "9.0.0", 1),
            ],
        };
        assert_eq!(pick(&index, "image.base64", 1).unwrap().version, "1.10.0");
        assert_eq!(pick(&index, "image.base64", 2).unwrap().version, "2.0.0");
        assert!(pick(&index, "image.base64", 3).is_none());
    }
}
