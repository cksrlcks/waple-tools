//! 테스트용: 도구 크레이트를 추가 설치용 DLL로 빌드한다 (릴리스 워크플로와 같은 명령).

use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

/// `cargo rustc -p <package> --features addon --crate-type cdylib`를 따로 둔 target 폴더에서 실행하고
/// 만들어진 DLL 경로를 돌려준다. 바깥 `cargo test`의 빌드 잠금과 겹치지 않게 target 폴더를 나눈다.
pub fn build_addon(package: &str) -> PathBuf {
    static ONE: Mutex<()> = Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = root.join("target/addon-test");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(&root)
        .args([
            "rustc",
            "-p",
            package,
            "--features",
            "addon",
            "--crate-type",
            "cdylib",
        ])
        .arg("--target-dir")
        .arg(&target)
        .status()
        .expect("cargo 실행 실패");
    assert!(status.success(), "{package} DLL 빌드 실패");
    let name = package.replace('-', "_");
    target.join("debug").join(format!(
        "{}{name}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}
