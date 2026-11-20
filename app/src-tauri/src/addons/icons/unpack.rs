//! npm tarball(.tgz)에서 필요한 파일만 메모리로 읽는다. 디스크에는 풀지 않는다.

use std::collections::HashMap;
use std::io::Read;

use flate2::read::GzDecoder;

const MAX_FILE: u64 = 64 * 1024 * 1024;

/// 항목 경로가 정확히 `package/<name>`인 것만 읽는다.
pub fn read_package_files(tgz: &[u8], names: &[&str]) -> Result<HashMap<String, Vec<u8>>, String> {
    let bad = || "받은 파일이 손상되었습니다. 다시 받아 주세요.".to_string();
    let mut archive = tar::Archive::new(GzDecoder::new(tgz));
    let mut out = HashMap::new();
    for entry in archive.entries().map_err(|_| bad())? {
        let entry = entry.map_err(|_| bad())?;
        let Ok(path) = entry.path() else { continue };
        let Some(name) = path
            .to_str()
            .and_then(|p| p.strip_prefix("package/"))
            .and_then(|n| names.iter().find(|w| **w == n))
        else {
            continue;
        };
        let name = name.to_string();
        if entry.size() > MAX_FILE {
            return Err("아이콘 세트 파일이 너무 큽니다.".into());
        }
        let mut buf = Vec::new();
        entry
            .take(MAX_FILE)
            .read_to_end(&mut buf)
            .map_err(|_| bad())?;
        out.insert(name, buf);
    }
    Ok(out)
}
