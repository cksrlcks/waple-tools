//! 디스크 배치: `<root>/<tool>/installed.json`, `<root>/<tool>/<id>/<version>/<files>`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const INSTALLED: &str = "installed.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstalledEntry {
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Installed {
    #[serde(default)]
    pub items: BTreeMap<String, InstalledEntry>,
}

/// 깨졌거나 없으면 빈 값
pub fn read_installed(tool_dir: &Path) -> Installed {
    std::fs::read(tool_dir.join(INSTALLED))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn write_installed(tool_dir: &Path, installed: &Installed) -> io::Result<()> {
    std::fs::create_dir_all(tool_dir)?;
    let tmp = tool_dir.join(format!("{INSTALLED}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(installed)?)?;
    std::fs::rename(tmp, tool_dir.join(INSTALLED))
}

pub fn item_dir(tool_dir: &Path, id: &str, version: &str) -> PathBuf {
    tool_dir.join(id).join(version)
}

/// 파일마다 임시 이름으로 쓰고 바꾼다.
pub fn write_files(dir: &Path, files: &[(String, Vec<u8>)]) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, data) in files {
        let tmp = dir.join(format!("{name}.tmp"));
        std::fs::write(&tmp, data)?;
        std::fs::rename(tmp, dir.join(name))?;
    }
    Ok(())
}
