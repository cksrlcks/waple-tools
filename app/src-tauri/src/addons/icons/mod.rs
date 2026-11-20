//! 아이콘 라이브러리: npm(@iconify-json/*)에서 세트를 받아 SVG로 보관한다.

pub mod npm;
pub mod sanitize;
pub mod set;
mod store;
pub mod unpack;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;

use super::net;
use super::store::safe_name;
use npm::{
    COLLECTIONS_PACKAGE, PackageMeta, latest_url, parse_meta, set_package, valid_prefix,
    verify_integrity,
};
use set::{Collection, IconSet, build_set, free_collections};
use store::{
    Installed, InstalledEntry, SET_FILE, read_installed, set_dir, write_installed, write_set,
};
use unpack::read_package_files;

const BROKEN: &str = "받은 파일이 손상되었습니다. 다시 받아 주세요.";
const BAD_PREFIX: &str = "세트 이름이 올바르지 않습니다.";
const SAVE_FAILED: &str = "아이콘 세트를 저장할 수 없습니다.";

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSet {
    pub prefix: String,
    pub name: String,
    pub version: String,
    /// 마지막 업데이트 확인에서 본 최신 버전 (현재와 다를 때만)
    pub latest: Option<String>,
}

pub struct IconSets {
    root: PathBuf,
    installed: Mutex<Installed>,
    latest: Mutex<HashMap<String, String>>,
    installing: Mutex<HashSet<String>>,
}

/// 받는 중 표시. 끝나면(실패 포함) 자동으로 뺀다.
struct Busy<'a>(&'a IconSets, String);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        lock(&self.0.installing).remove(&self.1);
    }
}

impl IconSets {
    pub fn from_env() -> Self {
        Self::open(waple_tools_builtin::app_data_dir().join("icons"))
    }

    pub fn open(root: PathBuf) -> Self {
        Self {
            installed: Mutex::new(read_installed(&root)),
            root,
            latest: Mutex::default(),
            installing: Mutex::default(),
        }
    }

    fn entry(&self, prefix: &str) -> Option<InstalledSet> {
        let e = lock(&self.installed).sets.get(prefix)?.clone();
        // 기록만 있고 폴더가 없으면 없는 것으로 본다
        if !safe_name(&e.version)
            || !set_dir(&self.root, prefix, &e.version)
                .join(SET_FILE)
                .is_file()
        {
            return None;
        }
        let latest = lock(&self.latest)
            .get(prefix)
            .filter(|l| **l != e.version)
            .cloned();
        Some(InstalledSet {
            prefix: prefix.to_string(),
            name: e.name,
            version: e.version,
            latest,
        })
    }

    pub fn list(&self) -> Vec<InstalledSet> {
        let prefixes: Vec<String> = lock(&self.installed).sets.keys().cloned().collect();
        prefixes.iter().filter_map(|p| self.entry(p)).collect()
    }

    fn begin(&self, prefix: &str) -> Result<Busy<'_>, String> {
        if !valid_prefix(prefix) {
            return Err(BAD_PREFIX.into());
        }
        if !lock(&self.installing).insert(prefix.to_string()) {
            return Err("이미 받는 중입니다.".into());
        }
        Ok(Busy(self, prefix.to_string()))
    }

    async fn fetch_meta(package: &str) -> Result<PackageMeta, String> {
        let bytes = net::fetch(&latest_url(package), None, |_, _| {}).await?;
        parse_meta(&bytes).ok_or_else(|| BROKEN.to_string())
    }

    /// 받기와 업데이트 공용.
    pub async fn install(&self, prefix: &str) -> Result<InstalledSet, String> {
        let _busy = self.begin(prefix)?;
        let meta = Self::fetch_meta(&set_package(prefix)).await?;
        if !safe_name(&meta.version) {
            return Err(BROKEN.into());
        }
        let tgz = net::fetch(&meta.tarball, None, |_, _| {}).await?;
        self.apply(prefix, meta, tgz)
    }

    /// 테스트용: 받기 뒤의 부분만 (진행 중 표시 포함)
    #[cfg(test)]
    async fn install_from(
        &self,
        prefix: &str,
        meta: PackageMeta,
        tgz: Vec<u8>,
    ) -> Result<InstalledSet, String> {
        let _busy = self.begin(prefix)?;
        self.apply(prefix, meta, tgz)
    }

    /// 무결성 확인 -> 변환 -> 저장
    fn apply(&self, prefix: &str, meta: PackageMeta, tgz: Vec<u8>) -> Result<InstalledSet, String> {
        if !safe_name(&meta.version) || !verify_integrity(&tgz, &meta.integrity) {
            return Err(BROKEN.into());
        }
        let files = read_package_files(&tgz, &["icons.json", "info.json", "metadata.json"])?;
        let set = build_set(prefix, &meta.version, &files)?;
        self.save(prefix, &set)?;
        lock(&self.latest).remove(prefix);
        self.entry(prefix).ok_or_else(|| SAVE_FAILED.to_string())
    }

    /// 새 버전 폴더에 쓰고 기록을 바꾼 뒤 예전 버전 폴더를 지운다. 실패하면 새 폴더만 지운다.
    fn save(&self, prefix: &str, set: &IconSet) -> Result<(), String> {
        let dir = set_dir(&self.root, prefix, &set.version);
        let mut inst = lock(&self.installed);
        let old = inst.sets.get(prefix).cloned();
        let saved = write_set(&dir, set).and_then(|_| {
            inst.sets.insert(
                prefix.to_string(),
                InstalledEntry {
                    name: set.name.clone(),
                    version: set.version.clone(),
                },
            );
            let r = write_installed(&self.root, &inst);
            if r.is_err() {
                match &old {
                    Some(o) => inst.sets.insert(prefix.to_string(), o.clone()),
                    None => inst.sets.remove(prefix),
                };
            }
            r
        });
        if saved.is_err() {
            // 같은 버전을 다시 받다 실패했다면 지우지 않는다
            if old.as_ref().is_none_or(|o| o.version != set.version) {
                let _ = std::fs::remove_dir_all(&dir);
            }
            return Err(SAVE_FAILED.into());
        }
        if let Some(o) = old.filter(|o| o.version != set.version && safe_name(&o.version)) {
            let _ = std::fs::remove_dir_all(set_dir(&self.root, prefix, &o.version));
        }
        Ok(())
    }

    /// 받은 세트마다 최신 버전만 확인한다.
    pub async fn check_updates(&self) -> Result<Vec<InstalledSet>, String> {
        let prefixes: Vec<String> = self.list().into_iter().map(|s| s.prefix).collect();
        for p in prefixes {
            let meta = Self::fetch_meta(&set_package(&p)).await?;
            lock(&self.latest).insert(p, meta.version);
        }
        Ok(self.list())
    }

    pub fn remove(&self, prefix: &str) -> Result<(), String> {
        if !valid_prefix(prefix) {
            return Err(BAD_PREFIX.into());
        }
        {
            let mut inst = lock(&self.installed);
            if inst.sets.remove(prefix).is_some() {
                write_installed(&self.root, &inst).map_err(|_| SAVE_FAILED.to_string())?;
            }
        }
        lock(&self.latest).remove(prefix);
        let dir = self.root.join(prefix);
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(|_| SAVE_FAILED.to_string())?;
        }
        Ok(())
    }

    pub fn read(&self, prefix: &str) -> Result<IconSet, String> {
        let none = || "받은 세트가 없습니다.".to_string();
        let s = self.entry(prefix).ok_or_else(none)?;
        let bytes = std::fs::read(set_dir(&self.root, prefix, &s.version).join(SET_FILE))
            .map_err(|_| none())?;
        serde_json::from_slice(&bytes).map_err(|_| BROKEN.to_string())
    }

    /// 무료 세트 목록. 디스크에 쓰지 않는다.
    pub async fn collections(&self) -> Result<Vec<Collection>, String> {
        let meta = Self::fetch_meta(COLLECTIONS_PACKAGE).await?;
        let tgz = net::fetch(&meta.tarball, None, |_, _| {}).await?;
        if !verify_integrity(&tgz, &meta.integrity) {
            return Err(BROKEN.into());
        }
        let files = read_package_files(&tgz, &["collections.json"])?;
        free_collections(files.get("collections.json").ok_or(BROKEN)?)
    }
}

#[cfg(test)]
mod tests;
