use std::path::Path;
use std::sync::Arc;

use crate::collections;
use crate::sanitize::sanitize_body;
use crate::set::{self, IconSet};

const EVIL_BODY: &str = r##"<path d="M0 0" onclick="x()"/><script>alert(1)</script><image href="https://e.x/a.png"/><path fill="url(https://e.x/#a)" d="M1 1"/><path fill="url(#g)" d="M2 2"/>"##;
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/fixtures/icons");

/// `<tmp>/<id>/<version>/`에 픽스처 세 파일을 놓는다. icons.json은 바꿔 넣을 수 있다.
fn layout(id: &str, version: &str, icons: Option<&str>) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(id).join(version);
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["icons.json", "info.json", "metadata.json"] {
        std::fs::copy(Path::new(FIXTURE).join("package").join(n), dir.join(n)).unwrap();
    }
    if let Some(j) = icons {
        std::fs::write(dir.join("icons.json"), j).unwrap();
    }
    tmp
}

fn names(s: &IconSet) -> Vec<&str> {
    s.icons.iter().map(|i| i.name.as_str()).collect()
}

fn load_with(id: &str, icons: &str) -> Arc<IconSet> {
    let tmp = layout(id, "1.0.0", Some(icons));
    set::load(tmp.path(), id, "1.0.0").unwrap()
}

#[test]
fn collections_snapshot_is_free_and_sorted() {
    let all = collections::all();
    assert!(all.iter().any(|c| c.id == "tabler") && all.iter().any(|c| c.id == "ri"));
    for c in all {
        assert!(
            ["MIT", "Apache-2.0", "ISC", "CC0-1.0"].contains(&c.license.as_str()),
            "{}",
            c.id
        );
        assert!(set::valid_name(&c.id), "{}", c.id);
    }
    assert!(all.windows(2).all(|w| w[0].name <= w[1].name));
}

#[test]
fn load_reads_fixture_set() {
    let tmp = layout("test-fixture", "1.0.0", None);
    let s = set::load(tmp.path(), "test-fixture", "1.0.0").unwrap();
    assert_eq!(
        (s.id.as_str(), s.name.as_str()),
        ("test-fixture", "Test Icons")
    );
    assert_eq!(names(&s), ["evil", "home", "home-filled", "small-20-solid"]);
    assert_eq!(s.suffixes.get("filled").map(String::as_str), Some("Filled"));
    assert_eq!(
        s.icons
            .iter()
            .find(|i| i.name == "small-20-solid")
            .unwrap()
            .width,
        Some(20.0)
    );
    assert_eq!((s.width, s.height), (24.0, 24.0));
    assert!(!s.palette);
}

#[test]
fn drops_invalid_names() {
    let s = load_with(
        "invalid-names",
        r#"{"icons":{
            "a\"><style>*{display:none}</style><x y=\"":{"body":"<mask id=\"m\"/>"},
            "Bad":{"body":"<path d=\"M0 0\"/>"},
            "ok-1":{"body":"<path d=\"M0 0\"/>"}}}"#,
    );
    assert_eq!(names(&s), ["ok-1"]);
}

#[test]
fn drops_empty_bodies() {
    let s = load_with(
        "empty-bodies",
        r#"{"icons":{"only-script":{"body":"<script>alert(1)</script>"},"ok":{"body":"<path d=\"M0 0\"/>"}}}"#,
    );
    assert_eq!(names(&s), ["ok"]);
}

#[test]
fn accepts_fractional_sizes() {
    let s = load_with(
        "fractional",
        r#"{"width":24.5,"icons":{"a":{"body":"<path d=\"M0 0\"/>","width":20.5}}}"#,
    );
    assert_eq!(s.width, 24.5);
    assert_eq!(s.icons[0].width, Some(20.5));
}

#[test]
fn missing_icons_json_is_error() {
    let tmp = layout("no-icons", "1.0.0", None);
    std::fs::remove_file(tmp.path().join("no-icons/1.0.0/icons.json")).unwrap();
    assert!(set::load(tmp.path(), "no-icons", "1.0.0").is_err());
}

#[test]
fn broken_metadata_is_ignored() {
    let tmp = layout("bad-meta", "1.0.0", None);
    std::fs::write(tmp.path().join("bad-meta/1.0.0/metadata.json"), "{").unwrap();
    assert!(
        set::load(tmp.path(), "bad-meta", "1.0.0")
            .unwrap()
            .suffixes
            .is_empty()
    );
}

#[test]
fn rejects_unsafe_id_and_version() {
    let tmp = layout("safe", "1.0.0", None);
    for (id, v) in [
        ("../x", "1.0.0"),
        ("Safe", "1.0.0"),
        ("safe", ".."),
        ("safe", "a/b"),
        ("safe", r"a\b"),
        ("safe", ".hidden"),
        ("safe", ""),
    ] {
        assert!(set::load(tmp.path(), id, v).is_err(), "{id} {v}");
    }
}

#[test]
fn second_run_uses_cache() {
    let tmp = layout("cached", "1.0.0", None);
    let a = set::load(tmp.path(), "cached", "1.0.0").unwrap();
    std::fs::remove_dir_all(tmp.path().join("cached")).unwrap();
    let b = set::load(tmp.path(), "cached", "1.0.0").unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn name_rules() {
    for ok in ["tabler", "ri", "material-symbols", "ic", "a1-2b"] {
        assert!(set::valid_name(ok), "{ok}");
    }
    for bad in ["", "-a", "a-", "a--b", "A", "../x", "a/b", "a.b", "a b"] {
        assert!(!set::valid_name(bad), "{bad}");
    }
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
