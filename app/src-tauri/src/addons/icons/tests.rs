use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::sanitize::sanitize_body;
use super::set::{build_set, free_collections};
use super::unpack::read_package_files;

const EVIL_BODY: &str = r##"<path d="M0 0" onclick="x()"/><script>alert(1)</script><image href="https://e.x/a.png"/><path fill="url(https://e.x/#a)" d="M1 1"/><path fill="url(#g)" d="M2 2"/>"##;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/icons")
        .join(name)
}

fn fixture_files() -> HashMap<String, Vec<u8>> {
    ["icons.json", "info.json", "metadata.json"]
        .iter()
        .map(|n| {
            (
                n.to_string(),
                std::fs::read(fixture(&format!("package/{n}"))).unwrap(),
            )
        })
        .collect()
}

fn fixture_tgz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (path, data) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, path, *data).unwrap();
    }
    let raw = tar.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&raw).unwrap();
    gz.finish().unwrap()
}

#[test]
fn reads_only_requested_package_files() {
    let tgz = fixture_tgz(&[
        ("package/index.js", b"js"),
        ("package/icons.json", b"right"),
        ("other/icons.json", b"wrong"),
    ]);
    let got = read_package_files(&tgz, &["icons.json"]).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got["icons.json"], b"right");
}

#[test]
fn rejects_non_gzip() {
    assert!(read_package_files(b"not a tgz", &["icons.json"]).is_err());
}

#[test]
fn sanitize_drops_script_handlers_and_external_refs() {
    let s = sanitize_body(EVIL_BODY).unwrap();
    assert!(
        !s.contains("script")
            && !s.contains("onclick")
            && !s.contains("<image")
            && !s.contains("https://")
    );
    assert!(s.contains(r#"fill="url(#g)""#) && s.contains(r#"d="M2 2""#));
}

#[test]
fn sanitize_keeps_stroke_icon_as_is_semantically() {
    let s = sanitize_body(
        r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12"/>"#,
    )
    .unwrap();
    for a in [
        r#"stroke-width="2""#,
        r#"stroke-linecap="round""#,
        r#"stroke-linejoin="round""#,
        r#"stroke="currentColor""#,
    ] {
        assert!(s.contains(a), "{a} missing in {s}");
    }
}

#[test]
fn sanitize_drops_css_escape_and_string_urls() {
    let s = sanitize_body(
        r##"<path fill="\75 rl(https://e.x/a)" d="M1 1"/><path mask="image-set('https://e.x/a.png' 1x)" d="M2 2"/>"##,
    )
    .unwrap();
    assert!(
        !s.contains("fill=") && !s.contains("mask=") && !s.contains("e.x"),
        "{s}"
    );
    assert!(s.contains(r#"d="M1 1""#) && s.contains(r#"d="M2 2""#));
}

#[test]
fn sanitize_rejects_broken_xml() {
    assert!(sanitize_body(r#"<path d="M0 0""#).is_none());
}

#[test]
fn sanitize_drops_namespaced_attrs_and_escapes() {
    let s = sanitize_body(
        r##"<g xmlns:xlink="http://www.w3.org/1999/xlink"><path xlink:fill="red" d="M&amp;&lt;1"/></g>"##,
    )
    .unwrap();
    assert!(!s.contains("fill="), "{s}");
    assert!(s.contains("&amp;") && s.contains("&lt;"), "{s}");
}

#[test]
fn build_set_from_fixture() {
    let set = build_set("test", "1.0.0", &fixture_files()).unwrap();
    assert_eq!(set.name, "Test Icons");
    assert_eq!(set.license.spdx.as_deref(), Some("MIT"));
    assert_eq!(
        set.icons
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>(),
        ["evil", "home", "home-filled", "small-20-solid"]
    );
    assert_eq!(
        set.suffixes.get("filled").map(String::as_str),
        Some("Filled")
    );
    assert_eq!(
        set.icons
            .iter()
            .find(|i| i.name == "small-20-solid")
            .unwrap()
            .width,
        Some(20.0)
    );
    assert_eq!((set.width, set.height), (24.0, 24.0));
}

fn set_from_icons_json(icons: &str) -> super::set::IconSet {
    let mut files = fixture_files();
    files.insert("icons.json".into(), icons.as_bytes().to_vec());
    build_set("test", "1.0.0", &files).unwrap()
}

#[test]
fn build_set_drops_invalid_names() {
    // 이름은 화면에서 id 치환 문자열에 들어가므로 iconify 이름 규칙이 아니면 뺀다.
    let set = set_from_icons_json(
        r#"{"icons":{
            "a\"><style>*{display:none}</style><x y=\"":{"body":"<mask id=\"m\"/>"},
            "Bad":{"body":"<path d=\"M0 0\"/>"},
            "ok-1":{"body":"<path d=\"M0 0\"/>"}}}"#,
    );
    let names: Vec<_> = set.icons.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["ok-1"]);
}

#[test]
fn build_set_drops_icons_with_empty_body() {
    let set = set_from_icons_json(
        r#"{"icons":{"only-script":{"body":"<script>alert(1)</script>"},"ok":{"body":"<path d=\"M0 0\"/>"}}}"#,
    );
    let names: Vec<_> = set.icons.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["ok"]);
}

#[test]
fn build_set_accepts_fractional_sizes() {
    let set = set_from_icons_json(
        r#"{"width":24.5,"icons":{"a":{"body":"<path d=\"M0 0\"/>","width":20.5}}}"#,
    );
    assert_eq!(set.width, 24.5);
    assert_eq!(set.icons[0].width, Some(20.5));
}

#[test]
fn build_set_requires_icons_json() {
    let mut files = fixture_files();
    files.remove("icons.json");
    assert!(build_set("test", "1.0.0", &files).is_err());
}

#[test]
fn free_collections_filters_license_and_hidden() {
    let c = free_collections(&std::fs::read(fixture("collections.json")).unwrap()).unwrap();
    assert_eq!(
        c.iter().map(|c| c.prefix.as_str()).collect::<Vec<_>>(),
        ["cc0-set", "isc-set", "mit-set", "apache-set"]
    );
}

// ---- 세트 저장소 ----

use base64::Engine as _;
use sha2::{Digest, Sha512};
use tauri::async_runtime::block_on;

use super::IconSets;
use super::npm::PackageMeta;

const BROKEN: &str = "받은 파일이 손상되었습니다. 다시 받아 주세요.";

fn package_tgz() -> Vec<u8> {
    let files = fixture_files();
    let entries: Vec<(String, &[u8])> = files
        .iter()
        .map(|(n, b)| (format!("package/{n}"), b.as_slice()))
        .collect();
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, b)| (n.as_str(), *b)).collect();
    fixture_tgz(&refs)
}

fn meta(version: &str, tgz: &[u8], good: bool) -> PackageMeta {
    let digest = if good {
        Sha512::digest(tgz)
    } else {
        Sha512::digest(b"other")
    };
    PackageMeta {
        version: version.into(),
        tarball: String::new(),
        integrity: format!(
            "sha512-{}",
            base64::engine::general_purpose::STANDARD.encode(digest)
        ),
    }
}

fn install(sets: &IconSets, version: &str, good: bool) -> Result<super::InstalledSet, String> {
    let tgz = package_tgz();
    block_on(sets.install_from("test", meta(version, &tgz, good), tgz))
}

#[test]
fn install_writes_set_and_record() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    let got = install(&sets, "1.0.0", true).unwrap();
    assert_eq!(
        (got.prefix.as_str(), got.version.as_str()),
        ("test", "1.0.0")
    );
    assert_eq!(sets.list(), [got]);
    assert_eq!(sets.read("test").unwrap().icons.len(), 4);
    assert!(dir.path().join("test/1.0.0/set.json").is_file());
    // 다시 열어도 기록이 남는다
    assert_eq!(IconSets::open(dir.path().to_path_buf()).list().len(), 1);
}

#[test]
fn bad_integrity_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    assert_eq!(install(&sets, "1.0.0", false).unwrap_err(), BROKEN);
    assert!(sets.list().is_empty());
    assert!(!dir.path().join("test").exists());
}

#[test]
fn update_replaces_and_removes_old() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    install(&sets, "1.0.0", true).unwrap();
    install(&sets, "1.1.0", true).unwrap();
    assert_eq!(sets.list()[0].version, "1.1.0");
    assert!(!dir.path().join("test/1.0.0").exists());
    assert!(dir.path().join("test/1.1.0/set.json").is_file());
}

#[test]
fn failed_update_keeps_old_version() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    install(&sets, "1.0.0", true).unwrap();
    assert!(install(&sets, "1.1.0", false).is_err());
    assert_eq!(sets.read("test").unwrap().version, "1.0.0");
    assert!(!dir.path().join("test/1.1.0").exists());
}

#[test]
fn install_twice_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    sets.installing.lock().unwrap().insert("test".into());
    assert_eq!(
        install(&sets, "1.0.0", true).unwrap_err(),
        "이미 받는 중입니다."
    );
    // 끝나면 풀려 다시 받을 수 있다
    sets.installing.lock().unwrap().clear();
    install(&sets, "1.0.0", true).unwrap();
    install(&sets, "1.0.0", true).unwrap();
}

#[test]
fn invalid_prefix_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    assert_eq!(
        block_on(sets.install("../x")).unwrap_err(),
        "세트 이름이 올바르지 않습니다."
    );
    assert!(sets.remove("../x").is_err());
}

#[test]
fn remove_deletes_folder_and_record() {
    let dir = tempfile::tempdir().unwrap();
    let sets = IconSets::open(dir.path().to_path_buf());
    install(&sets, "1.0.0", true).unwrap();
    sets.remove("test").unwrap();
    assert!(sets.list().is_empty());
    assert!(!dir.path().join("test").exists());
    assert_eq!(sets.read("test").unwrap_err(), "받은 세트가 없습니다.");
    assert!(IconSets::open(dir.path().to_path_buf()).list().is_empty());
}

#[test]
fn broken_installed_json_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("installed.json"), "{").unwrap();
    assert!(IconSets::open(dir.path().to_path_buf()).list().is_empty());
}

#[test]
fn missing_folder_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("installed.json"),
        r#"{"sets":{"test":{"name":"Test","version":"1.0.0"}}}"#,
    )
    .unwrap();
    assert!(IconSets::open(dir.path().to_path_buf()).list().is_empty());
}
