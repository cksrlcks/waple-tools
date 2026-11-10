//! mozjpeg-sys에 들어 있는 jpegtran 변환 코드(transupp.c)와 우리 래퍼를 함께 컴파일한다.
//! mozjpeg-sys가 `links = "jpeg"`로 내보내는 include 경로(DEP_JPEG_INCLUDE)를 쓴다.

use std::env;
use std::path::PathBuf;

fn main() {
    let include =
        env::var_os("DEP_JPEG_INCLUDE").expect("mozjpeg-sys가 include 경로를 내보내지 않았습니다");
    let dirs: Vec<PathBuf> = env::split_paths(&include).collect();
    let transupp = dirs
        .iter()
        .map(|d| d.join("transupp.c"))
        .find(|p| p.exists())
        .expect("transupp.c를 찾을 수 없습니다");
    cc::Build::new()
        .includes(&dirs)
        .file(&transupp)
        .file("src/lt_transform.c")
        .warnings(false)
        .compile("lt_transform");
    println!("cargo:rerun-if-changed=src/lt_transform.c");
}
