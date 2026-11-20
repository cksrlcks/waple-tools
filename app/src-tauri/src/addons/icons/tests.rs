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
        Some(20)
    );
    assert_eq!((set.width, set.height), (24, 24));
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
