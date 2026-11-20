//! npm 레지스트리 메타데이터와 tarball 무결성(sha512) 확인.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use sha2::{Digest, Sha512};

pub const COLLECTIONS_PACKAGE: &str = "@iconify/collections";

/// iconify prefix: 소문자·숫자를 `-`로 이은 것 (`^[a-z0-9]+(-[a-z0-9]+)*$`).
pub fn valid_prefix(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

pub fn latest_url(package: &str) -> String {
    format!("https://registry.npmjs.org/{package}/latest")
}

pub fn set_package(prefix: &str) -> String {
    format!("@iconify-json/{prefix}")
}

pub struct PackageMeta {
    pub version: String,
    pub tarball: String,
    pub integrity: String,
}

#[derive(Deserialize)]
struct RawMeta {
    version: String,
    dist: RawDist,
}

#[derive(Deserialize)]
struct RawDist {
    tarball: String,
    integrity: String,
}

pub fn parse_meta(bytes: &[u8]) -> Option<PackageMeta> {
    let m: RawMeta = serde_json::from_slice(bytes).ok()?;
    Some(PackageMeta {
        version: m.version,
        tarball: m.dist.tarball,
        integrity: m.dist.integrity,
    })
}

/// `sha512-<base64>` 형식만 인정한다.
pub fn verify_integrity(bytes: &[u8], integrity: &str) -> bool {
    integrity
        .strip_prefix("sha512-")
        .and_then(|b64| STANDARD.decode(b64).ok())
        .is_some_and(|want| want == Sha512::digest(bytes).as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_rules() {
        for ok in [
            "tabler",
            "ri",
            "material-symbols",
            "fluent-emoji-flat",
            "ic",
        ] {
            assert!(valid_prefix(ok), "{ok}");
        }
        for bad in ["", "-a", "a-", "a--b", "A", "../x", "a/b", "a.b", "a b"] {
            assert!(!valid_prefix(bad), "{bad}");
        }
    }

    #[test]
    fn parses_latest_meta() {
        let m = parse_meta(br#"{"version":"1.2.46","dist":{"tarball":"https://registry.npmjs.org/@iconify-json/tabler/-/tabler-1.2.46.tgz","integrity":"sha512-AAAA"}}"#).unwrap();
        assert_eq!(m.version, "1.2.46");
        assert!(m.tarball.ends_with("tabler-1.2.46.tgz"));
    }

    #[test]
    fn integrity_sha512_only() {
        let b = b"hello";
        let good = format!("sha512-{}", STANDARD.encode(Sha512::digest(b)));
        assert!(verify_integrity(b, &good));
        assert!(!verify_integrity(b"other", &good));
        assert!(!verify_integrity(b, "sha1-abc"));
    }
}
