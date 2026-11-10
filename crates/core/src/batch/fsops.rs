//! 덮어쓰지 않는 이동과 해시. Windows의 `fs::rename`은 대상이 있으면 덮어쓰므로 이동은 반드시 `move_new`로.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

// ponytail: 전역 잠금 — 커밋(이동)만 직렬화, 도구 실행은 병렬. 이동은 ms 단위라 병목 아님
static COMMIT_LOCK: Mutex<()> = Mutex::new(());

/// `want`가 비어 있으면 그대로, 있으면 `stem (1).ext`, `stem (2).ext` … 중 처음 비어 있는 경로.
pub fn unique_path(want: &Path) -> PathBuf {
    if !want.exists() {
        return want.to_path_buf();
    }
    let stem = want.file_stem().unwrap_or_default().to_string_lossy();
    let ext = want
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|n| want.with_file_name(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("무한 반복자")
}

/// `from`을 `want`(겹치면 번호 붙인 이름)로 옮기고 실제 경로를 돌려준다. 기존 파일은 절대 덮어쓰지 않는다.
pub fn move_new(from: &Path, want: &Path) -> io::Result<PathBuf> {
    let _guard = COMMIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(parent) = want.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let to = unique_path(want);
    if std::fs::rename(from, &to).is_err() {
        // 다른 드라이브로 옮길 때는 rename이 안 되므로 복사 후 삭제
        std::fs::copy(from, &to)?;
        std::fs::remove_file(from)?;
    }
    Ok(to)
}

/// 파일 sha256 (소문자 hex)
pub fn file_hash(p: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(p)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_numbers_until_free() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.txt"), "").unwrap();
        std::fs::write(d.path().join("a (1).txt"), "").unwrap();
        assert_eq!(
            unique_path(&d.path().join("a.txt")),
            d.path().join("a (2).txt")
        );
        std::fs::write(d.path().join("README"), "").unwrap();
        assert_eq!(
            unique_path(&d.path().join("README")),
            d.path().join("README (1)")
        );
        assert_eq!(
            unique_path(&d.path().join("new.txt")),
            d.path().join("new.txt")
        );
    }

    #[test]
    fn move_new_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src.txt");
        let dst = d.path().join("dst.txt");
        std::fs::write(&src, "new").unwrap();
        std::fs::write(&dst, "old").unwrap();
        let moved = move_new(&src, &dst).unwrap();
        assert_eq!(moved, d.path().join("dst (1).txt"));
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "old");
        assert_eq!(std::fs::read_to_string(&moved).unwrap(), "new");
        assert!(!src.exists());
    }

    #[test]
    fn move_new_creates_parent_dirs() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("s.txt");
        std::fs::write(&src, "x").unwrap();
        let moved = move_new(&src, &d.path().join("x/y/z.txt")).unwrap();
        assert_eq!(std::fs::read_to_string(moved).unwrap(), "x");
    }

    #[test]
    fn file_hash_is_sha256() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a");
        std::fs::write(&p, "abc").unwrap();
        assert_eq!(
            file_hash(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
