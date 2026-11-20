use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde_json::json;
use sha2::{Digest, Sha512};
use tauri::async_runtime::block_on;

use super::npm::PackageMeta;
use super::spec::{self, ResourceSpec};
use super::unpack::read_package_files;
use super::{ResourceState, ToolResources};

const BROKEN: &str = "받은 파일이 손상되었습니다. 다시 받아 주세요.";
const TOOL: &str = "web.icon_library";

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

fn schema(package: &str, files: serde_json::Value) -> serde_json::Value {
    json!({"x-resources": {"source": "npm", "package": package, "files": files}})
}

#[test]
fn spec_parses_and_rejects() {
    let ok = spec::parse(&schema("@scope/{id}", json!(["icons.json", "info.json"]))).unwrap();
    assert_eq!(ok.package, "@scope/{id}");
    assert_eq!(ok.files, ["icons.json", "info.json"]);

    assert!(spec::parse(&json!({})).is_none());
    assert!(spec::parse(&schema("@scope/pkg", json!(["a.json"]))).is_none());
    assert!(spec::parse(&schema("@s/{id}-{id}", json!(["a.json"]))).is_none());
    assert!(spec::parse(&schema("@s/{id}", json!([]))).is_none());
    assert!(spec::parse(&schema("@s/{id}", json!(["../x"]))).is_none());
    assert!(spec::parse(&schema("@s/{id}", json!(["a/b.json"]))).is_none());
    // 윈도우 드라이브 상대 경로·대체 데이터 스트림·역슬래시
    for bad in ["C:x", "a:b", r"a\b", "a.json:ads", ".hidden"] {
        assert!(
            spec::parse(&schema("@s/{id}", json!([bad]))).is_none(),
            "{bad}"
        );
    }
    let mut other = schema("@s/{id}", json!(["a.json"]));
    other["x-resources"]["source"] = json!("http");
    assert!(spec::parse(&other).is_none());
}

#[test]
fn spec_defaults_drop_invalid_ids() {
    let mut s = schema("@s/{id}", json!(["a.json"]));
    assert!(spec::parse(&s).unwrap().defaults.is_empty());
    s["x-resources"]["defaults"] = json!(["lucide", "Bad", "../x", 3, "ri", "", "a--b"]);
    assert_eq!(spec::parse(&s).unwrap().defaults, ["lucide", "ri"]);
    // 배열이 아니면 없는 것으로 본다 (스펙은 그대로 쓸 수 있다)
    s["x-resources"]["defaults"] = json!("lucide");
    assert!(spec::parse(&s).unwrap().defaults.is_empty());
}

#[test]
fn pending_defaults_skip_installed_and_duplicates() {
    let mut sp = test_spec();
    sp.defaults = vec!["a".into(), "b".into(), "a".into(), "c".into()];
    let installed = |ids: &[&str]| -> Vec<ResourceState> {
        ids.iter()
            .map(|id| ResourceState {
                id: id.to_string(),
                version: "1.0.0".into(),
                latest: None,
            })
            .collect()
    };
    assert_eq!(sp.pending_defaults(&installed(&[])), ["a", "b", "c"]);
    assert_eq!(sp.pending_defaults(&installed(&["b", "x"])), ["a", "c"]);
    assert!(sp.pending_defaults(&installed(&["a", "b", "c"])).is_empty());
}

#[test]
fn tarball_must_be_npm_registry() {
    use super::tarball_ok;
    assert!(tarball_ok("https://registry.npmjs.org/@s/p/-/p-1.0.0.tgz"));
    assert!(!tarball_ok("http://registry.npmjs.org/p.tgz"));
    assert!(!tarball_ok("https://github.com/o/r/archive/x.tgz"));
    assert!(!tarball_ok("https://registry.npmjs.org.evil.com/p.tgz"));
    assert!(!tarball_ok("https://x.githubusercontent.com/p.tgz"));
    assert!(!tarball_ok("not a url"));
}

fn test_spec() -> ResourceSpec {
    ResourceSpec {
        package: "@scope/{id}".into(),
        files: vec!["icons.json".into(), "info.json".into()],
        defaults: vec![],
    }
}

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

fn install(res: &ToolResources, version: &str, good: bool) -> Result<ResourceState, String> {
    let tgz = package_tgz();
    block_on(res.install_from(TOOL, &test_spec(), "test", meta(version, &tgz, good), tgz))
}

fn open(dir: &tempfile::TempDir) -> ToolResources {
    ToolResources::open(dir.path().to_path_buf())
}

#[test]
fn install_writes_under_tool_dir() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    let got = install(&res, "1.0.0", true).unwrap();
    assert_eq!((got.id.as_str(), got.version.as_str()), ("test", "1.0.0"));
    assert_eq!(res.list(TOOL), [got]);
    let base = dir.path().join(TOOL);
    assert!(base.join("test/1.0.0/icons.json").is_file());
    assert!(base.join("test/1.0.0/info.json").is_file());
    // 스펙에 없는 파일은 저장하지 않는다
    assert!(!base.join("test/1.0.0/metadata.json").exists());
    let rec: serde_json::Value =
        serde_json::from_slice(&std::fs::read(base.join("installed.json")).unwrap()).unwrap();
    assert_eq!(rec, json!({"items": {"test": {"version": "1.0.0"}}}));
    // 다시 열어도 기록이 남는다
    assert_eq!(open(&dir).list(TOOL).len(), 1);
    // 다른 도구에는 보이지 않는다
    assert!(res.list("other.tool").is_empty());
}

#[test]
fn missing_package_file_is_broken() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    let tgz = fixture_tgz(&[("package/icons.json", b"{}")]);
    let m = meta("1.0.0", &tgz, true);
    let err = block_on(res.install_from(TOOL, &test_spec(), "test", m, tgz)).unwrap_err();
    assert_eq!(err, BROKEN);
    assert!(res.list(TOOL).is_empty());
}

#[test]
fn injected_lists_installed() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    assert_eq!(
        res.injected(TOOL),
        json!({"dir": dir.path().join(TOOL), "installed": {}})
    );
    install(&res, "1.0.0", true).unwrap();
    assert_eq!(
        res.injected(TOOL),
        json!({"dir": dir.path().join(TOOL), "installed": {"test": "1.0.0"}})
    );
}

#[test]
fn remove_tool_deletes_folder() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    install(&res, "1.0.0", true).unwrap();
    res.remove_tool(TOOL);
    assert!(!dir.path().join(TOOL).exists());
    assert!(res.list(TOOL).is_empty());
}

#[test]
fn remove_deletes_item_and_record() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    install(&res, "1.0.0", true).unwrap();
    res.remove(TOOL, "test").unwrap();
    assert!(res.list(TOOL).is_empty());
    assert!(!dir.path().join(TOOL).join("test").exists());
    assert!(open(&dir).list(TOOL).is_empty());
}

#[test]
fn invalid_id_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    let bad = "리소스 이름이 올바르지 않습니다.";
    assert_eq!(
        block_on(res.install(TOOL, &test_spec(), "../x")).unwrap_err(),
        bad
    );
    assert_eq!(res.remove(TOOL, "../x").unwrap_err(), bad);
    // 도구 id도 경로에 쓰기 전에 거른다
    assert_eq!(
        block_on(res.install("../x", &test_spec(), "test")).unwrap_err(),
        bad
    );
    assert!(res.list("../x").is_empty());
}

#[test]
fn bad_integrity_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    assert_eq!(install(&res, "1.0.0", false).unwrap_err(), BROKEN);
    assert!(res.list(TOOL).is_empty());
    assert!(!dir.path().join(TOOL).join("test").exists());
}

#[test]
fn unsafe_version_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    assert_eq!(install(&res, "../evil", true).unwrap_err(), BROKEN);
    assert!(res.list(TOOL).is_empty());
}

#[test]
fn update_replaces_and_removes_old() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    install(&res, "1.0.0", true).unwrap();
    install(&res, "1.1.0", true).unwrap();
    assert_eq!(res.list(TOOL)[0].version, "1.1.0");
    let base = dir.path().join(TOOL).join("test");
    assert!(!base.join("1.0.0").exists());
    assert!(base.join("1.1.0/icons.json").is_file());
}

#[test]
fn failed_update_keeps_old_version() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    install(&res, "1.0.0", true).unwrap();
    assert!(install(&res, "1.1.0", false).is_err());
    assert_eq!(res.list(TOOL)[0].version, "1.0.0");
    assert!(!dir.path().join(TOOL).join("test/1.1.0").exists());
}

#[test]
fn install_twice_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    res.installing
        .lock()
        .unwrap()
        .insert((TOOL.into(), "test".into()));
    assert_eq!(
        install(&res, "1.0.0", true).unwrap_err(),
        "이미 받는 중입니다."
    );
    // 끝나면 풀려 다시 받을 수 있다
    res.installing.lock().unwrap().clear();
    install(&res, "1.0.0", true).unwrap();
    install(&res, "1.0.0", true).unwrap();
}

#[test]
fn second_hold_is_refused_and_does_not_clear_first() {
    let dir = tempfile::tempdir().unwrap();
    let res = open(&dir);
    let first = res.hold(TOOL).unwrap();
    assert!(res.hold(TOOL).is_none()); // 거부된 시도는 표시를 남기지도 지우지도 않는다
    assert!(res.hold(TOOL).is_none());
    assert!(res.hold("other.tool").is_some());
    drop(first);
    assert!(res.hold(TOOL).is_some());
}

#[test]
fn broken_installed_json_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(TOOL)).unwrap();
    std::fs::write(dir.path().join(TOOL).join("installed.json"), "{").unwrap();
    assert!(open(&dir).list(TOOL).is_empty());
}

#[test]
fn missing_folder_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(TOOL)).unwrap();
    std::fs::write(
        dir.path().join(TOOL).join("installed.json"),
        r#"{"items":{"test":{"version":"1.0.0"}}}"#,
    )
    .unwrap();
    assert!(open(&dir).list(TOOL).is_empty());
}
