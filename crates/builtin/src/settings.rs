//! 앱 설정. `%LOCALAPPDATA%\LocalTools\settings.json` 한 파일에 둔다.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use localtools_core::batch::OutputPolicy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: Theme,
    /// 0이면 CPU 코어 수 - 1
    pub threads: usize,
    pub default_policy: OutputPolicy,
    pub keep_if_larger: bool,
    /// 이미지 도구의 메타데이터 기본값 (false = 지우기)
    pub keep_metadata: bool,
    /// 0이면 정리하지 않음
    pub log_retention_days: u32,
    pub favorites: Vec<String>,
    /// 도구 id → 마지막으로 쓴 옵션
    pub last_options: BTreeMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            threads: 0,
            default_policy: OutputPolicy::default(),
            keep_if_larger: true,
            keep_metadata: false,
            log_retention_days: 30,
            favorites: vec![],
            last_options: BTreeMap::new(),
        }
    }
}

pub fn settings_path() -> PathBuf {
    crate::app_data_dir().join("settings.json")
}

/// 없거나 깨졌으면 기본값. 빠진 필드는 기본값으로 채운다.
pub fn load_settings(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("[localtools] 설정 파일을 읽을 수 없어 기본값을 씁니다: {e}");
        Settings::default()
    })
}

/// 임시 파일에 쓴 뒤 바꿔 넣는다 — 쓰다가 꺼져도 반쯤 쓴 설정 파일이 남지 않는다.
pub fn save_settings(path: &Path, s: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(s).map_err(io::Error::other)?;
    std::fs::write(&tmp, json)?;
    // 자기 설정 파일을 바꾸는 것이라 덮어쓰기 rename이 맞다
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_settings_defaults_when_missing() {
        let d = tempfile::tempdir().unwrap();
        let s = load_settings(&d.path().join("none.json"));
        assert_eq!(s, Settings::default());
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.log_retention_days, 30);
        assert!(s.keep_if_larger);
        assert_eq!(s.threads, 0);
        assert!(!s.keep_metadata, "기본은 메타데이터 지우기");
    }

    #[test]
    fn load_settings_falls_back_on_corrupt_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, "{not json").unwrap();
        assert_eq!(load_settings(&p), Settings::default());
    }

    #[test]
    fn load_settings_fills_missing_fields() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, r#"{"theme":"dark"}"#).unwrap();
        let s = load_settings(&p);
        assert_eq!(s.theme, Theme::Dark);
        assert_eq!(s.log_retention_days, 30);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/settings.json");
        let s = Settings {
            favorites: vec!["dev.copy".into()],
            last_options: [("dev.copy".to_string(), serde_json::json!({"q": 1}))].into(),
            theme: Theme::Light,
            ..Default::default()
        };
        save_settings(&p, &s).unwrap();
        assert_eq!(load_settings(&p), s);
        // camelCase로 저장되는지
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(raw.contains("logRetentionDays"), "{raw}");
    }
}
