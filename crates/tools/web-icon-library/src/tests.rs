use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};
use waple_tools_core::{Block, JobContext, ToolKind, ToolOutput};

use crate::collections;
use crate::sanitize::sanitize_body;
use crate::set::{self, Icon, IconSet};
use crate::{png, render, styles, tool};

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
    // 앱의 store::safe_name과 같게 '+'는 받지 않는다 (폴더가 있어도)
    let plus = layout("plus", "1.0.0+build", None);
    assert_eq!(
        set::load(plus.path(), "plus", "1.0.0+build").unwrap_err(),
        "리소스 이름이 올바르지 않습니다."
    );
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

// ---- 도구 (옵션 · 블록 · 렌더) ----

fn run(opts: Value) -> Vec<Block> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    match tool().run(&ctx, &[], out.path(), &opts).unwrap() {
        ToolOutput::Blocks(b) => {
            assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
            b
        }
        ToolOutput::Files(_) => panic!("블록이어야 함"),
    }
}

/// 픽스처 세트를 `test`/`1.0.0`으로 받아 둔 것처럼 `__resources`를 붙인다
fn with_fixture(mut opts: Value) -> (tempfile::TempDir, Value) {
    let tmp = layout("test", "1.0.0", None);
    opts["set"] = json!("test");
    opts["__resources"] = json!({ "dir": tmp.path(), "installed": { "test": "1.0.0" } });
    (tmp, opts)
}

fn types(blocks: &[Block]) -> Vec<String> {
    blocks
        .iter()
        .map(|b| {
            serde_json::to_value(b).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn icon(name: &str) -> Icon {
    Icon {
        name: name.into(),
        body: r#"<path stroke="currentColor" stroke-width="2" d="M0 0"/>"#.into(),
        width: None,
        height: None,
    }
}

fn test_set(id: &str) -> IconSet {
    IconSet {
        id: id.into(),
        name: "Test".into(),
        version: "1.0.0".into(),
        palette: false,
        width: 24.0,
        height: 24.0,
        suffixes: BTreeMap::new(),
        icons: vec![],
    }
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn code_named<'a>(blocks: &'a [Block], lang: &str) -> Option<(&'a str, Option<&'a str>)> {
    blocks.iter().find_map(|b| match b {
        Block::Code {
            lang: l,
            code,
            name,
        } if l == lang => Some((code.as_str(), name.as_deref())),
        _ => None,
    })
}

#[test]
fn schema_flags() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["x-panel-when"], "selected");
    assert_eq!(s["x-panel-blocks"], json!(["image", "code"]));
    assert_eq!(s["x-stage"], "list-detail");
    assert_eq!(s["x-stage-plain"], true);
    assert_eq!(
        s["x-resources"],
        json!({ "source": "npm", "package": "@iconify-json/{id}", "files": ["icons.json", "info.json", "metadata.json"] })
    );
    let p = &s["properties"];
    for k in ["set", "style", "query", "selected", "limit"] {
        assert_eq!(p[k]["x-hidden"], true, "{k}");
    }
    assert!(p.get("__resources").is_none());
    assert_eq!(p["stroke"]["x-step"], 0.25);
    assert_eq!(p["color"]["format"], "color");
    assert_eq!(p["color"]["x-enabled-when"], json!({ "useColor": [true] }));
    assert_eq!(p["limit"]["default"], 400);
    assert_eq!(p["pngScale"]["default"], "x2");
}

#[test]
fn meta_name_matches_catalog() {
    let cat: Vec<Value> =
        serde_json::from_str(include_str!("../../../../catalog/tools.json")).unwrap();
    let m = tool().meta();
    let e = cat.iter().find(|e| e["id"] == m.id.as_str()).unwrap();
    assert_eq!(e["name"], m.name.as_str());
    assert_eq!(m.name, "아이콘 라이브러리");
    assert_eq!(tool().kind(), ToolKind::Interactive);
}

#[test]
fn no_resources_lists_collections_only() {
    let b = run(json!({}));
    assert_eq!(types(&b), ["resourceList"]);
    let Block::ResourceList {
        items,
        option,
        resets,
        selected,
    } = &b[0]
    else {
        unreachable!()
    };
    assert_eq!(items[0].id, collections::all()[0].id);
    assert_eq!((option.as_str(), selected), ("set", &None));
    assert_eq!(resets, &["selected", "query", "style", "limit"]);
    let tabler = items.iter().find(|i| i.id == "tabler").unwrap();
    assert!(tabler.meta.ends_with("개 · MIT"), "{}", tabler.meta);
}

#[test]
fn missing_resource_dir_shows_download_text() {
    let b = run(json!({ "set": "test", "__resources": { "dir": "nowhere", "installed": {} } }));
    assert_eq!(types(&b), ["resourceList", "text"]);
    assert!(matches!(&b[0], Block::ResourceList { selected: Some(s), .. } if s == "test"));
    assert!(matches!(&b[1], Block::Text { text }
        if text == "목록에서 [받기]를 누르면 아이콘을 볼 수 있습니다."));
}

#[test]
fn grid_respects_limit_and_more() {
    let (_tmp, opts) = with_fixture(json!({ "limit": 2 }));
    let b = run(opts);
    assert_eq!(types(&b), ["resourceList", "iconGrid"]);
    let Block::IconGrid {
        cells,
        option,
        selected,
        chips,
        search,
        more,
        total,
    } = &b[1]
    else {
        unreachable!()
    };
    assert_eq!(cells.len(), 2);
    // 픽스처 suffixes {"", filled} → 기본 칩 ""에서 home-filled는 빠진다 (evil·home·small-20-solid)
    assert_eq!(*total, 3);
    let more = more.as_ref().unwrap();
    assert_eq!(
        (more.option.as_str(), more.value, more.label.as_str()),
        ("limit", 402, "더 보기")
    );
    assert_eq!((option.as_str(), selected), ("selected", &None));
    let chips = chips.as_ref().unwrap();
    assert_eq!((chips.option.as_str(), chips.value.as_str()), ("style", ""));
    assert_eq!(chips.choices.len(), 2);
    assert_eq!(chips.resets, ["limit"]);
    let search = search.as_ref().unwrap();
    assert_eq!(
        (search.option.as_str(), search.placeholder.as_str()),
        ("query", "아이콘 검색")
    );
    assert_eq!(search.resets, ["limit"]);
    assert_eq!(cells[0].key, cells[0].title);
    assert!(cells.iter().all(|c| c.svg.contains(r#"height="24""#)));

    let (_tmp, opts) = with_fixture(json!({ "style": "filled" }));
    let b = run(opts);
    let Block::IconGrid {
        cells, more, total, ..
    } = &b[1]
    else {
        unreachable!()
    };
    assert_eq!((cells.len(), *total, more.is_none()), (1, 1, true));
    assert_eq!(cells[0].key, "home-filled");
}

#[test]
fn unsuffixed_icons_show_in_every_chip() {
    let icons: Vec<Icon> = ["bold", "home-line", "home-fill"].map(icon).into();
    let ri = map(&[("line", "선"), ("fill", "채움")]);
    let names = |style| -> Vec<&str> {
        styles::filter(&icons, "", Some(style), &ri)
            .iter()
            .map(|i| i.name.as_str())
            .collect()
    };
    assert_eq!(names("line"), ["bold", "home-line"]);
    assert_eq!(names("fill"), ["bold", "home-fill"]);

    // 1판 filter-icons 테스트: 스타일·검색어(모든 단어, 대소문자 무시)
    let icons: Vec<Icon> = ["home", "home-filled", "arrow-left", "arrow-left-filled"]
        .map(icon)
        .into();
    let tb = styles::styles_of(&test_set("tabler"));
    let names = |q, style| -> Vec<&str> {
        styles::filter(&icons, q, style, &tb)
            .iter()
            .map(|i| i.name.as_str())
            .collect()
    };
    assert_eq!(names("", Some("")), ["home", "arrow-left"]);
    assert_eq!(names("LEFT arrow", Some("filled")), ["arrow-left-filled"]);
    assert_eq!(names("home", None).len(), 2);

    // 키가 하나뿐이면 거르지 않는다
    let one: Vec<Icon> = ["bold", "home-outline"].map(icon).into();
    let os = map(&[("outline", "외곽선")]);
    assert_eq!(styles::filter(&one, "", Some("outline"), &os).len(), 2);
    assert_eq!(styles::filter(&one, "", None, &os).len(), 2);
}

#[test]
fn styles_of_and_style_of_match_v1() {
    let hero = map(&[
        ("", "선 24"),
        ("solid", "채움 24"),
        ("20-solid", "채움 20"),
        ("16-solid", "채움 16"),
    ]);
    assert_eq!(styles::style_of("academic-cap-20-solid", &hero), "20-solid");
    assert_eq!(styles::style_of("academic-cap-solid", &hero), "solid");
    assert_eq!(styles::style_of("academic-cap", &hero), "");
    let tb = styles::styles_of(&test_set("tabler"));
    assert_eq!(tb, map(&[("", "선"), ("filled", "채움")]));
    assert_eq!(styles::style_of("home-filled", &tb), "filled");
    let mut other = test_set("other");
    assert!(styles::styles_of(&other).is_empty());
    other.suffixes = map(&[("line", "Line")]);
    assert_eq!(styles::styles_of(&other), other.suffixes);
}

#[test]
fn selected_produces_panel_blocks() {
    let (_tmp, opts) = with_fixture(json!({ "selected": "home" }));
    let b = run(opts);
    assert_eq!(
        types(&b),
        ["resourceList", "iconGrid", "image", "code", "code", "code"]
    );
    assert!(matches!(&b[1], Block::IconGrid { selected: Some(s), .. } if s == "home"));
    let Block::Image { src, name } = &b[2] else {
        unreachable!()
    };
    assert_eq!(name, "home.png");
    assert!(src.starts_with("data:image/png;base64,"));
    let (svg, name) = code_named(&b, "svg").unwrap();
    assert_eq!(name, Some("home.svg"));
    assert!(
        svg.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">"#
        ),
        "{svg}"
    );
    assert!(svg.contains(r#"stroke-width="2""#) && svg.contains("currentColor"));
    let (jsx, _) = code_named(&b, "jsx").unwrap();
    assert!(jsx.contains(r#"strokeWidth="2""#), "{jsx}");
    let (uri, _) = code_named(&b, "data uri").unwrap();
    assert!(
        uri.starts_with(r#"url("data:image/svg+xml,%3Csvg"#),
        "{uri}"
    );
    assert!(code_named(&b, "class").is_none());

    // 크기·두께·색 옵션
    let (_tmp, opts) = with_fixture(json!({
        "selected": "home", "size": 48, "stroke": 1.5, "useColor": true, "color": "#ff0000"
    }));
    let b = run(opts);
    let (svg, _) = code_named(&b, "svg").unwrap();
    assert!(
        svg.contains(r#"width="48" height="48""#) && svg.contains(r#"stroke-width="1.5""#),
        "{svg}"
    );
    assert!(
        svg.contains("#ff0000") && !svg.contains("currentColor"),
        "{svg}"
    );

    // 색 지정이 꺼져 있으면 color 값은 무시
    let (_tmp, opts) = with_fixture(json!({ "selected": "home", "color": "#ff0000" }));
    let b = run(opts);
    assert!(code_named(&b, "svg").unwrap().0.contains("currentColor"));
}

#[test]
fn bad_color_is_reported_not_injected() {
    let (_tmp, opts) = with_fixture(json!({
        "selected": "home", "useColor": true, "color": "red\"/><script>"
    }));
    let b = run(opts);
    assert!(code_named(&b, "svg").is_none());
    assert!(
        b.iter()
            .any(|b| matches!(b, Block::Text { text } if text.contains("#rrggbb")))
    );
}

#[test]
fn unknown_selected_gives_no_panel_blocks() {
    // home-filled는 기본 칩("")에서 걸러진다
    for sel in ["nope", "home-filled"] {
        let (_tmp, opts) = with_fixture(json!({ "selected": sel }));
        let b = run(opts);
        assert_eq!(types(&b), ["resourceList", "iconGrid"], "{sel}");
        assert!(matches!(&b[1], Block::IconGrid { selected: None, .. }));
    }
}

#[test]
fn svg_output_matches_v1() {
    let set = test_set("test");
    let i = icon("home");
    assert_eq!(
        render::svg(&i, &set, 48.0, Some(1.5), "currentColor"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><path stroke="currentColor" stroke-width="1.5" d="M0 0"/></svg>"#
    );
    assert!(
        render::svg(&i, &set, 24.0, None, "#ff0000")
            .contains(r##"stroke="#ff0000" stroke-width="2""##)
    );
    let wide = Icon {
        width: Some(48.0),
        height: Some(24.0),
        ..icon("wide")
    };
    assert!(
        render::svg(&wide, &set, 24.0, None, "currentColor")
            .contains(r#"width="48" height="24" viewBox="0 0 48 24""#)
    );
    let frac = Icon {
        width: Some(20.5),
        height: Some(20.0),
        ..icon("frac")
    };
    assert!(
        render::svg(&frac, &set, 20.0, None, "currentColor")
            .contains(r#"width="21" height="20" viewBox="0 0 20.5 20""#)
    );
}

#[test]
fn copy_formats_match_v1() {
    assert_eq!(
        render::jsx(r#"<path stroke-width="2" stroke-linecap="round" data-x="1"/>"#),
        r#"<path strokeWidth="2" strokeLinecap="round" data-x="1"/>"#
    );
    assert_eq!(
        render::jsx(r#"<path aria-hidden="true"/>"#),
        r#"<path aria-hidden="true"/>"#
    );
    assert_eq!(
        render::jsx("<g\tclip-path=\"url(#a)\">"),
        r#"<g clipPath="url(#a)">"#
    );
    assert_eq!(
        render::data_uri("<svg/>"),
        r#"url("data:image/svg+xml,%3Csvg%2F%3E")"#
    );
    assert_eq!(
        render::data_uri("a b#'()*~\u{e9}"),
        "url(\"data:image/svg+xml,a%20b%23'()*~%C3%A9\")"
    );
}

#[test]
fn png_size_scales() {
    let svg = render::svg(&icon("home"), &test_set("test"), 24.0, None, "currentColor");
    let bytes = png::render(&svg, 2).unwrap();
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    assert_eq!((be(16), be(20)), (48, 48));
    // currentColor는 검정으로 그린다
    let filled = r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><path fill="currentColor" d="M0 0h4v4H0z"/></svg>"#;
    let pm = resvg::tiny_skia::Pixmap::decode_png(&png::render(filled, 1).unwrap()).unwrap();
    let px = pm.pixel(1, 1).unwrap();
    assert_eq!(
        (px.red(), px.green(), px.blue(), px.alpha()),
        (0, 0, 0, 255)
    );
}

#[test]
fn class_snippets() {
    assert_eq!(
        render::class_snippet("tabler", "home").as_deref(),
        Some(r#"<i class="ti ti-home"></i>"#)
    );
    assert_eq!(
        render::class_snippet("ri", "home-line").as_deref(),
        Some(r#"<i class="ri-home-line"></i>"#)
    );
    assert_eq!(render::class_snippet("heroicons", "home"), None);
    assert_eq!(render::class_snippet("test", "home"), None);
}
