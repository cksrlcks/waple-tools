//! 추가 설치 도구: 원격 목록 → 다운로드 → sha256·서명 확인 → 버전 폴더 → DLL 로드 (설계 문서 6.5~6.7).

mod index;
mod net;
mod store;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use minisign_verify::PublicKey;
use serde::Serialize;
use waple_tools_addon_abi::{ABI_VERSION, load};
use waple_tools_core::Tool;

use index::{Index, Installed, InstalledTool, cmp_version, pick};
use store::{
    cleanup, parse_key, read_installed, safe_name, sha256_hex, verified_dll, verify, version_dir,
    write_installed, write_version,
};

/// `~/.waple-tools/addon-signing.key.pub` (앱 업데이트 키와 다른 키). 비밀키는 GitHub Secrets `ADDON_SIGNING_PRIVATE_KEY`.
const RELEASE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDkzQzg3REQxMUFCMTcxQTMKUldTamNiRWEwWDNJay9yY3hsY0lVUnBkYk1xUk1FY3BzSThEcjJMdmJGVWEzY1BVck1HenRXNFQK";

const TAMPERED: &str = "설치 파일이 손상되었거나 변조되었습니다.";
const SAVE_FAILED: &str = "도구 파일을 저장할 수 없습니다.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AddonStatus {
    Installed,
    NotInstalled,
    /// 같은 ABI의 새 버전이 목록에 있다
    UpdateAvailable,
    /// 설치는 됐지만 읽을 수 없다 (앱 ABI가 바뀜, 파일이 바뀜). 다시 설치해야 한다
    UpdateRequired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonInfo {
    pub status: AddonStatus,
    /// 설치된 버전
    pub version: Option<String>,
    /// 원격 목록의 이 앱용 최신 버전과 다운로드 크기
    pub latest: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub received: u64,
    pub total: Option<u64>,
}

pub struct Addons {
    root: PathBuf,
    keys: Vec<PublicKey>,
    /// debug 빌드: 로컬 index.json (환경변수 `WAPLE_ADDON_INDEX`)
    local_index: Option<PathBuf>,
    loaded: RwLock<HashMap<String, Arc<dyn Tool>>>,
    installed: Mutex<Installed>,
    remote: Mutex<Option<Index>>,
    installing: Mutex<HashSet<String>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Addons {
    /// 앱에서 쓰는 설정: `%LOCALAPPDATA%\waple-tools\addons`, 배포 공개키.
    /// debug 빌드에서만 `WAPLE_ADDON_PUBKEY`(개발용 공개키)와 `WAPLE_ADDON_INDEX`(로컬 index.json)를 받는다.
    pub fn from_env() -> Self {
        let mut keys: Vec<_> = parse_key(RELEASE_PUBKEY).into_iter().collect();
        let mut local_index = None;
        if cfg!(debug_assertions) {
            keys.extend(
                std::env::var("WAPLE_ADDON_PUBKEY")
                    .ok()
                    .and_then(|k| parse_key(&k)),
            );
            local_index = std::env::var_os("WAPLE_ADDON_INDEX").map(PathBuf::from);
        }
        Self::open(
            waple_tools_builtin::app_data_dir().join("addons"),
            keys,
            local_index,
        )
    }

    /// 예전 판 폴더를 지우고, 서명을 다시 확인한 DLL만 연다.
    pub fn open(root: PathBuf, keys: Vec<PublicKey>, local_index: Option<PathBuf>) -> Self {
        let installed = read_installed(&root);
        cleanup(&root, &installed);
        let mut loaded = HashMap::new();
        for (id, t) in &installed.tools {
            if t.abi != ABI_VERSION || !safe_name(id) || !safe_name(&t.version) {
                continue;
            }
            match open_dll(&version_dir(&root, id, &t.version), id, &keys) {
                Ok(tool) => {
                    loaded.insert(id.clone(), tool);
                }
                Err(e) => eprintln!("[waple-tools] 추가 도구 {id} 읽기 실패: {e}"),
            }
        }
        Self {
            root,
            keys,
            local_index,
            loaded: RwLock::new(loaded),
            installed: Mutex::new(installed),
            remote: Mutex::new(None),
            installing: Mutex::default(),
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.loaded
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
    }

    pub fn info(&self, id: &str) -> AddonInfo {
        let version = lock(&self.installed)
            .tools
            .get(id)
            .map(|t| t.version.clone());
        let remote = lock(&self.remote);
        let latest = remote.as_ref().and_then(|i| pick(i, id, ABI_VERSION));
        let status = match (&version, self.get(id).is_some()) {
            (None, _) => AddonStatus::NotInstalled,
            (Some(_), false) => AddonStatus::UpdateRequired,
            (Some(v), true) if latest.is_some_and(|l| cmp_version(&l.version, v).is_gt()) => {
                AddonStatus::UpdateAvailable
            }
            (Some(_), true) => AddonStatus::Installed,
        };
        AddonInfo {
            status,
            version,
            latest: latest.map(|l| l.version.clone()),
            size: latest.map(|l| l.size),
        }
    }

    /// 원격 목록을 받아 둔다 (업데이트 표시용)
    pub async fn refresh(&self) -> Result<(), String> {
        let index = self.fetch_index().await?;
        *lock(&self.remote) = Some(index);
        Ok(())
    }

    fn local_base(&self) -> Option<&Path> {
        self.local_index.as_deref().and_then(Path::parent)
    }

    async fn fetch_index(&self) -> Result<Index, String> {
        let url = match &self.local_index {
            Some(p) => p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            None => net::INDEX_URL.to_string(),
        };
        let bytes = net::fetch(&url, self.local_base(), |_, _| {}).await?;
        serde_json::from_slice(&bytes).map_err(|_| "도구 목록을 읽을 수 없습니다.".to_string())
    }

    /// 설치·업데이트·다시 설치. 새 판은 새 버전 폴더에 두고, 예전 판 폴더는 다음 시작 때 지운다.
    pub async fn install(
        &self,
        id: &str,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> Result<(), String> {
        if !lock(&self.installing).insert(id.to_string()) {
            return Err("이미 설치 중입니다.".into());
        }
        let result = async {
            let index = self.fetch_index().await?;
            *lock(&self.remote) = Some(index.clone());
            let entry = pick(&index, id, ABI_VERSION).cloned().ok_or_else(|| {
                if index.tools.iter().any(|e| e.id == id) {
                    "이 도구는 새 버전의 앱에서만 설치할 수 있습니다. 앱을 업데이트해 주세요."
                } else {
                    "설치할 수 있는 도구 파일이 없습니다."
                }
                .to_string()
            })?;
            if !safe_name(id) || !safe_name(&entry.version) {
                return Err(TAMPERED.into());
            }
            let installed = lock(&self.installed).tools.get(id).cloned();
            if installed
                .as_ref()
                .is_some_and(|t| t.version == entry.version)
                && self.get(id).is_some()
            {
                return Ok(()); // 이미 최신. 열린 DLL은 덮어쓸 수 없다
            }

            let bytes = net::fetch(&entry.url, self.local_base(), |received, total| {
                on_progress(DownloadProgress {
                    received,
                    total: total.or(Some(entry.size)),
                })
            })
            .await?;
            if sha256_hex(&bytes) != entry.sha256.to_ascii_lowercase()
                || !verify(&bytes, &entry.signature, &self.keys)
            {
                return Err(TAMPERED.into());
            }
            let dir = version_dir(&self.root, id, &entry.version);
            write_version(&dir, &bytes, &entry.signature).map_err(|_| SAVE_FAILED.to_string())?;
            let tool = match open_dll(&dir, id, &self.keys) {
                Ok(t) => t,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(e);
                }
            };
            {
                let mut inst = lock(&self.installed);
                inst.tools.insert(
                    id.to_string(),
                    InstalledTool {
                        version: entry.version.clone(),
                        abi: ABI_VERSION,
                    },
                );
                write_installed(&self.root, &inst).map_err(|_| SAVE_FAILED.to_string())?;
            }
            self.loaded
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id.to_string(), tool);
            Ok(())
        }
        .await;
        lock(&self.installing).remove(id);
        result
    }

    /// 기록에서 빼고 목록에서 내린다. 열린 DLL 파일은 다음 시작 때 지운다.
    pub fn uninstall(&self, id: &str) -> Result<(), String> {
        let mut inst = lock(&self.installed);
        if inst.tools.remove(id).is_none() {
            return Ok(());
        }
        write_installed(&self.root, &inst)
            .map_err(|_| "설치 기록을 저장할 수 없습니다.".to_string())?;
        self.loaded
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        Ok(())
    }
}

/// 서명 확인 → DLL 열기 → 도구 id가 맞는지 확인
fn open_dll(dir: &Path, id: &str, keys: &[PublicKey]) -> Result<Arc<dyn Tool>, String> {
    let dll = verified_dll(dir, keys).ok_or(TAMPERED)?;
    // SAFETY: 우리 키로 서명이 확인된 파일만 연다
    let tool = unsafe { load(&dll) }.map_err(|e| e.to_string())?;
    if tool.meta().id != id {
        return Err("도구 파일이 올바르지 않습니다.".into());
    }
    Ok(Arc::new(tool))
}
