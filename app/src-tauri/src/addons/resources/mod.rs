//! 도구 리소스: 추가 도구가 `x-resources`로 선언한 npm 패키지의 파일을 받아 도구별로 보관한다.
//! 사용자가 받기·업데이트를 누를 때만 네트워크를 쓴다.

pub mod npm;
pub mod spec;
mod store;
pub mod unpack;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use serde_json::{Value, json};

use super::net;
use super::store::safe_name;
use npm::{PackageMeta, latest_url, package_for, parse_meta, valid_id, verify_integrity};
use spec::ResourceSpec;
use store::{InstalledEntry, item_dir, read_installed, write_files, write_installed};
use unpack::read_package_files;

const BROKEN: &str = "받은 파일이 손상되었습니다. 다시 받아 주세요.";
const BAD_ID: &str = "리소스 이름이 올바르지 않습니다.";
const SAVE_FAILED: &str = "리소스를 저장할 수 없습니다.";

type Key = (String, String);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn key(tool_id: &str, id: &str) -> Key {
    (tool_id.to_string(), id.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceState {
    pub id: String,
    pub version: String,
    /// 마지막 업데이트 확인에서 본 최신 버전 (현재와 다를 때만)
    pub latest: Option<String>,
}

pub struct ToolResources {
    root: PathBuf,
    /// 설치 기록을 읽고 바꾸는 동안 한 번에 하나만
    write: Mutex<()>,
    latest: Mutex<HashMap<Key, String>>,
    installing: Mutex<HashSet<Key>>,
}

/// 받는 중 표시. 끝나면(실패 포함) 자동으로 뺀다.
struct Busy<'a>(&'a ToolResources, Key);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        lock(&self.0.installing).remove(&self.1);
    }
}

impl ToolResources {
    pub fn from_env() -> Self {
        Self::open(waple_tools_builtin::app_data_dir().join("addon-data"))
    }

    pub fn open(root: PathBuf) -> Self {
        Self {
            root,
            write: Mutex::default(),
            latest: Mutex::default(),
            installing: Mutex::default(),
        }
    }

    /// 경로에 쓰는 도구 id는 안전한 이름만
    fn tool_dir(&self, tool_id: &str) -> Option<PathBuf> {
        safe_name(tool_id).then(|| self.root.join(tool_id))
    }

    fn state(
        &self,
        tool_id: &str,
        dir: &Path,
        id: &str,
        e: &InstalledEntry,
    ) -> Option<ResourceState> {
        // 기록만 있고 폴더가 없으면 없는 것으로 본다
        if !valid_id(id) || !safe_name(&e.version) || !item_dir(dir, id, &e.version).is_dir() {
            return None;
        }
        let latest = lock(&self.latest)
            .get(&key(tool_id, id))
            .filter(|l| **l != e.version)
            .cloned();
        Some(ResourceState {
            id: id.to_string(),
            version: e.version.clone(),
            latest,
        })
    }

    pub fn list(&self, tool_id: &str) -> Vec<ResourceState> {
        let Some(dir) = self.tool_dir(tool_id) else {
            return vec![];
        };
        read_installed(&dir)
            .items
            .iter()
            .filter_map(|(id, e)| self.state(tool_id, &dir, id, e))
            .collect()
    }

    /// 도구에 넘길 위치와 받은 목록 (`opts["__resources"]`)
    pub fn injected(&self, tool_id: &str) -> Value {
        let installed: HashMap<String, String> = self
            .list(tool_id)
            .into_iter()
            .map(|s| (s.id, s.version))
            .collect();
        json!({ "dir": self.root.join(tool_id), "installed": installed })
    }

    fn begin(&self, tool_id: &str, id: &str) -> Result<Busy<'_>, String> {
        if !valid_id(id) || self.tool_dir(tool_id).is_none() {
            return Err(BAD_ID.into());
        }
        if !lock(&self.installing).insert(key(tool_id, id)) {
            return Err("이미 받는 중입니다.".into());
        }
        Ok(Busy(self, key(tool_id, id)))
    }

    async fn fetch_meta(package: &str) -> Result<PackageMeta, String> {
        let bytes = net::fetch(&latest_url(package), None, |_, _| {}).await?;
        parse_meta(&bytes).ok_or_else(|| BROKEN.to_string())
    }

    /// 받기와 업데이트 공용.
    pub async fn install(
        &self,
        tool_id: &str,
        spec: &ResourceSpec,
        id: &str,
    ) -> Result<ResourceState, String> {
        let _busy = self.begin(tool_id, id)?;
        let meta = Self::fetch_meta(&package_for(spec, id)).await?;
        if !safe_name(&meta.version) {
            return Err(BROKEN.into());
        }
        let tgz = net::fetch(&meta.tarball, None, |_, _| {}).await?;
        self.apply(tool_id, spec, id, meta, tgz)
    }

    /// 테스트용: 받기 뒤의 부분만 (진행 중 표시 포함)
    #[cfg(test)]
    async fn install_from(
        &self,
        tool_id: &str,
        spec: &ResourceSpec,
        id: &str,
        meta: PackageMeta,
        tgz: Vec<u8>,
    ) -> Result<ResourceState, String> {
        let _busy = self.begin(tool_id, id)?;
        self.apply(tool_id, spec, id, meta, tgz)
    }

    /// 무결성 확인 -> 필요한 파일 꺼내기 -> 저장
    fn apply(
        &self,
        tool_id: &str,
        spec: &ResourceSpec,
        id: &str,
        meta: PackageMeta,
        tgz: Vec<u8>,
    ) -> Result<ResourceState, String> {
        if !safe_name(&meta.version) || !verify_integrity(&tgz, &meta.integrity) {
            return Err(BROKEN.into());
        }
        let names: Vec<&str> = spec.files.iter().map(String::as_str).collect();
        let mut got = read_package_files(&tgz, &names)?;
        let files = spec
            .files
            .iter()
            .map(|n| got.remove(n).map(|b| (n.clone(), b)))
            .collect::<Option<Vec<_>>>()
            .ok_or(BROKEN)?;
        self.save(tool_id, id, &meta.version, &files)?;
        lock(&self.latest).remove(&key(tool_id, id));
        self.list(tool_id)
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| SAVE_FAILED.to_string())
    }

    /// 새 버전 폴더에 쓰고 기록을 바꾼 뒤 예전 버전 폴더를 지운다. 실패하면 새 폴더만 지운다.
    fn save(
        &self,
        tool_id: &str,
        id: &str,
        version: &str,
        files: &[(String, Vec<u8>)],
    ) -> Result<(), String> {
        let tool_dir = self.tool_dir(tool_id).ok_or(BAD_ID)?;
        let dir = item_dir(&tool_dir, id, version);
        let _w = lock(&self.write);
        let mut inst = read_installed(&tool_dir);
        let old = inst.items.get(id).cloned();
        let saved = write_files(&dir, files).and_then(|_| {
            inst.items.insert(
                id.to_string(),
                InstalledEntry {
                    version: version.to_string(),
                },
            );
            write_installed(&tool_dir, &inst)
        });
        if saved.is_err() {
            // 같은 버전을 다시 받다 실패했다면 지우지 않는다
            if old.as_ref().is_none_or(|o| o.version != version) {
                let _ = std::fs::remove_dir_all(&dir);
            }
            return Err(SAVE_FAILED.into());
        }
        if let Some(o) = old.filter(|o| o.version != version && safe_name(&o.version)) {
            let _ = std::fs::remove_dir_all(item_dir(&tool_dir, id, &o.version));
        }
        Ok(())
    }

    /// 받은 리소스마다 최신 버전만 확인한다.
    pub async fn check(
        &self,
        tool_id: &str,
        spec: &ResourceSpec,
    ) -> Result<Vec<ResourceState>, String> {
        for s in self.list(tool_id) {
            let meta = Self::fetch_meta(&package_for(spec, &s.id)).await?;
            lock(&self.latest).insert(key(tool_id, &s.id), meta.version);
        }
        Ok(self.list(tool_id))
    }

    pub fn remove(&self, tool_id: &str, id: &str) -> Result<(), String> {
        let dir = self
            .tool_dir(tool_id)
            .filter(|_| valid_id(id))
            .ok_or(BAD_ID)?;
        {
            let _w = lock(&self.write);
            let mut inst = read_installed(&dir);
            if inst.items.remove(id).is_some() {
                write_installed(&dir, &inst).map_err(|_| SAVE_FAILED.to_string())?;
            }
        }
        lock(&self.latest).remove(&key(tool_id, id));
        let item = dir.join(id);
        if item.exists() {
            std::fs::remove_dir_all(item).map_err(|_| SAVE_FAILED.to_string())?;
        }
        Ok(())
    }

    /// 도구를 지울 때 받은 리소스를 폴더째 지운다.
    pub fn remove_tool(&self, tool_id: &str) {
        if let Some(dir) = self.tool_dir(tool_id) {
            let _w = lock(&self.write);
            let _ = std::fs::remove_dir_all(dir);
            lock(&self.latest).retain(|(t, _), _| t != tool_id);
        }
    }
}

#[cfg(test)]
mod tests;
