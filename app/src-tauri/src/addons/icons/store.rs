//! 디스크 배치: `<root>/installed.json`, `<root>/<prefix>/<version>/set.json`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::set::IconSet;

const INSTALLED: &str = "installed.json";
pub const SET_FILE: &str = "set.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstalledEntry {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Installed {
    #[serde(default)]
    pub sets: BTreeMap<String, InstalledEntry>,
}

/// 깨졌거나 없으면 빈 값
pub fn read_installed(root: &Path) -> Installed {
    std::fs::read(root.join(INSTALLED))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn write_installed(root: &Path, installed: &Installed) -> io::Result<()> {
    std::fs::create_dir_all(root)?;
    let tmp = root.join(format!("{INSTALLED}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(installed)?)?;
    std::fs::rename(tmp, root.join(INSTALLED))
}

pub fn set_dir(root: &Path, prefix: &str, version: &str) -> PathBuf {
    root.join(prefix).join(version)
}

pub fn write_set(dir: &Path, set: &IconSet) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{SET_FILE}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec(set)?)?;
    std::fs::rename(tmp, dir.join(SET_FILE))
}
