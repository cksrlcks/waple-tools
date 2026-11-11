//! 덮어쓰지 않는 이동과 해시. Windows의 `fs::rename`은 대상이 있으면 덮어쓰므로 이동은 반드시 `move_new`로.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

// ponytail: 전역 잠금 — 커밋(이동+기록)만 직렬화, 도구 실행은 병렬. 이동은 ms 단위라 병목 아님.
// 같은 프로세스 안에서만 막는다: 다른 프로세스가 unique_path와 rename 사이에 같은 이름을 만들면 덮어쓸 수 있음
// (같은 볼륨이면 hard_link + remove로 원자화 가능, FAT은 미지원)
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

/// 새 폴더 `want`(있으면 `want (1)`, `want (2)` …)를 만들고 실제 경로를 돌려준다.
/// `create_dir`가 이미 있으면 실패하므로 병렬로 불러도 같은 폴더를 두 번 고르지 않는다.
pub fn create_new_dir(want: &Path) -> io::Result<PathBuf> {
    if let Some(parent) = want.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = want.file_name().unwrap_or_default().to_string_lossy();
    for n in 0.. {
        let p = if n == 0 {
            want.to_path_buf()
        } else {
            want.with_file_name(format!("{name} ({n})"))
        };
        match std::fs::create_dir(&p) {
            Ok(()) => return Ok(p),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    unreachable!("무한 반복")
}

/// `from`을 `want`(겹치면 번호 붙인 이름)로 옮기고 실제 경로를 돌려준다. 기존 파일은 절대 덮어쓰지 않는다.
pub fn move_new(from: &Path, want: &Path) -> io::Result<PathBuf> {
    move_new_logged(from, want, |_| Ok(()))
}

/// `move_new` + 같은 잠금 안에서 `record(옮긴 경로)` 호출.
/// 기록이 실패하면 방금 옮긴 파일을 `from`으로 되돌리고 오류를 낸다 — 기록 없는 이동은 남지 않는다.
/// 잠금 안에서 기록하므로 기록 순서 = 실제 이동 순서 (되돌리기가 역순으로 안전하게 적용됨).
pub fn move_new_logged(
    from: &Path,
    want: &Path,
    record: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let _guard = COMMIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(parent) = want.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let to = unique_path(want);
    rename_or_copy(from, &to)?;
    if let Err(e) = record(&to) {
        rename_or_copy(&to, from)?;
        return Err(e);
    }
    Ok(to)
}

/// 새 파일 `want`(겹치면 번호 붙인 이름)에 `bytes`를 쓰고 실제 경로를 돌려준다. 기존 파일은 절대 덮어쓰지 않는다
/// (`create_new`라 다른 프로세스가 사이에 같은 이름을 만들면 덮어쓰지 않고 실패한다).
pub fn write_new(want: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    use std::io::Write as _;
    let _guard = COMMIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let to = unique_path(want);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&to)?
        .write_all(bytes)?;
    Ok(to)
}

/// rename. 다른 드라이브라 rename이 안 될 때만 복사 후 삭제하고, 삭제가 안 되면 복사본을 지운다.
fn rename_or_copy(from: &Path, to: &Path) -> io::Result<()> {
    const ERROR_NOT_SAME_DEVICE: i32 = 17;
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e)
            if e.kind() == io::ErrorKind::CrossesDevices
                || e.raw_os_error() == Some(ERROR_NOT_SAME_DEVICE) =>
        {
            std::fs::copy(from, to)?;
            if let Err(e) = std::fs::remove_file(from) {
                let _ = std::fs::remove_file(to);
                return Err(e);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
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
    fn write_new_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        let want = d.path().join("qr.png");
        assert_eq!(write_new(&want, b"one").unwrap(), want);
        let second = write_new(&want, b"two").unwrap();
        assert_eq!(second, d.path().join("qr (1).png"));
        assert_eq!(std::fs::read(&want).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");
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
    fn move_new_logged_rolls_back_when_record_fails() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("s.txt");
        std::fs::write(&src, "x").unwrap();
        let r = move_new_logged(&src, &d.path().join("t.txt"), |_| {
            Err(io::Error::other("기록 실패"))
        });
        assert!(r.is_err());
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "x");
        assert!(!d.path().join("t.txt").exists());
    }

    #[test]
    fn record_runs_inside_commit_lock() {
        use std::sync::{Arc, Mutex};
        use std::time::Duration;
        let d = tempfile::tempdir().unwrap();
        let (x, y) = (d.path().join("x"), d.path().join("y"));
        std::fs::write(&x, "").unwrap();
        std::fs::write(&y, "").unwrap();
        let order = Arc::new(Mutex::new(Vec::new()));
        let (o, dir) = (order.clone(), d.path().to_path_buf());
        let t = std::thread::spawn(move || {
            move_new_logged(&x, &dir.join("x2"), |_| {
                std::thread::sleep(Duration::from_millis(300));
                o.lock().unwrap().push("A-record");
                Ok(())
            })
            .unwrap();
        });
        std::thread::sleep(Duration::from_millis(50));
        move_new(&y, &d.path().join("y2")).unwrap();
        order.lock().unwrap().push("B-moved");
        t.join().unwrap();
        assert_eq!(*order.lock().unwrap(), ["A-record", "B-moved"]);
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
