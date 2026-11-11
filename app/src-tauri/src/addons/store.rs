//! 서명 확인과 디스크 배치: `<root>/<id>/<version>/tool.dll` + `tool.dll.sig`, `<root>/installed.json`.

use std::io;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use minisign_verify::{PublicKey, Signature};
use sha2::{Digest, Sha256};

use super::index::Installed;

pub const DLL: &str = "tool.dll";
pub const SIG: &str = "tool.dll.sig";
const INSTALLED: &str = "installed.json";

fn b64_text(s: &str) -> Option<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// `tauri signer generate`의 `.pub` 내용 (base64로 감싼 minisign 공개키)
pub fn parse_key(b64: &str) -> Option<PublicKey> {
    PublicKey::decode(&b64_text(b64)?).ok()
}

/// 키 중 하나로 서명이 맞는가. `sig_b64`는 `tauri signer sign`의 `.sig` 내용.
pub fn verify(bytes: &[u8], sig_b64: &str, keys: &[PublicKey]) -> bool {
    let Some(sig) = b64_text(sig_b64).and_then(|t| Signature::decode(&t).ok()) else {
        return false;
    };
    keys.iter().any(|k| k.verify(bytes, &sig, true).is_ok())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 폴더 이름으로 써도 안전한가 (`..`·구분자·드라이브 없이). index.json은 서명이 없어서 꼭 거른다.
pub fn safe_name(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub fn version_dir(root: &Path, id: &str, version: &str) -> PathBuf {
    root.join(id).join(version)
}

/// 깨졌거나 없으면 빈 목록
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

/// 확인을 마친 DLL과 서명을 버전 폴더에 쓴다.
pub fn write_version(dir: &Path, dll: &[u8], sig_b64: &str) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(SIG), sig_b64)?;
    let tmp = dir.join(format!("{DLL}.tmp"));
    std::fs::write(&tmp, dll)?;
    std::fs::rename(tmp, dir.join(DLL))
}

/// 시작할 때: 서명이 맞는 DLL 경로. 설치 뒤 바꿔치기됐으면 None.
pub fn verified_dll(dir: &Path, keys: &[PublicKey]) -> Option<PathBuf> {
    let dll = dir.join(DLL);
    let bytes = std::fs::read(&dll).ok()?;
    let sig = std::fs::read_to_string(dir.join(SIG)).ok()?;
    verify(&bytes, &sig, keys).then_some(dll)
}

/// 시작할 때(아직 아무 DLL도 열지 않았을 때): 설치 기록에 없는 도구·버전 폴더를 지운다.
/// Windows는 열린 DLL을 지울 수 없어서, 업데이트·제거한 예전 판은 여기서 지운다.
pub fn cleanup(root: &Path, installed: &Installed) {
    let Ok(ids) = std::fs::read_dir(root) else {
        return;
    };
    for id in ids.flatten().filter(|e| e.path().is_dir()) {
        let name = id.file_name().to_string_lossy().into_owned();
        let keep = installed.tools.get(&name).map(|t| t.version.as_str());
        let Ok(versions) = std::fs::read_dir(id.path()) else {
            continue;
        };
        for v in versions.flatten() {
            if Some(v.file_name().to_string_lossy().as_ref()) != keep {
                let _ = std::fs::remove_dir_all(v.path());
            }
        }
        let _ = std::fs::remove_dir(id.path()); // 비었을 때만 지워진다
    }
}
