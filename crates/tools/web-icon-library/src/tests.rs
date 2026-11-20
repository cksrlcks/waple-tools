use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use base64::Engine as _;
use serde_json::{Value, json};
use waple_tools_core::{Action, ActionItem, Block, JobContext, ToolKind, ToolOutput};

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
    list(pairs).into_iter().collect()
}

fn list(pairs: &[(&str, &str)]) -> styles::Styles {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn actions(blocks: &[Block]) -> &[ActionItem] {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Actions { items } => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

fn labels(blocks: &[Block]) -> Vec<&str> {
    actions(blocks).iter().map(|i| i.label.as_str()).collect()
}

/// 이름이 `label`인 복사·저장 버튼의 글
fn text_of<'a>(blocks: &'a [Block], label: &str) -> &'a str {
    match &actions(blocks)
        .iter()
        .find(|i| i.label == label)
        .unwrap()
        .action
    {
        Action::CopyText { text } | Action::SaveText { text, .. } => text,
        Action::CopyImage { .. } => panic!("글이 아님"),
    }
}

#[test]
fn schema_flags() {
    let s = tool().options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["x-panel-when"], "selected");
    assert_eq!(s["x-panel-blocks"], json!(["image", "actions"]));
    assert_eq!(s["x-panel-position"], "bottom");
    assert_eq!(s["x-stage"], "list-detail");
    assert_eq!(s["x-stage-plain"], true);
    assert_eq!(
        s["x-resources"],
        json!({ "source": "npm", "package": "@iconify-json/{id}", "files": ["icons.json", "info.json", "metadata.json"] })
    );
    let p = &s["properties"];
    for k in ["set", "style", "query", "selected"] {
        assert_eq!(p[k]["x-hidden"], true, "{k}");
    }
    assert!(p.get("__resources").is_none());
    assert_eq!(p["stroke"]["x-step"], 0.25);
    assert_eq!(p["color"]["format"], "color");
    assert_eq!(
        p["color"]["x-swatches"],
        json!(["#000000", "#6B7280", "#2563EB", "#DC2626"])
    );
    assert_eq!(p["color"]["default"], "#000000");
    assert_eq!(p["size"]["maximum"], 512);
    assert_eq!(p["size"]["minimum"], 8);
    // 크기·두께는 null(원래대로) 허용 + 직접 입력칸
    for k in ["size", "stroke"] {
        assert!(p[k]["default"].is_null(), "{k}");
        assert!(
            p[k]["type"]
                .as_array()
                .is_some_and(|t| t.contains(&json!("null"))),
            "{}",
            p[k]
        );
        assert_eq!(p[k]["x-slider-input"], true, "{k}");
    }
    assert!(p.get("limit").is_none());
    assert!(p.get("useColor").is_none() && p.get("pngScale").is_none());
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
        check_label,
    } = &b[0]
    else {
        unreachable!()
    };
    assert_eq!(items[0].id, collections::all()[0].id);
    assert_eq!((option.as_str(), selected), ("set", &None));
    assert_eq!(resets, &["selected", "query", "style"]);
    assert_eq!(check_label.as_deref(), Some("아이콘 업데이트 확인"));
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
fn grid_sends_all_filtered_cells() {
    let (_tmp, opts) = with_fixture(json!({}));
    let b = run(opts);
    assert_eq!(types(&b), ["resourceList", "iconGrid"]);
    let Block::IconGrid {
        cells,
        option,
        selected,
        chips,
        search,
        total,
    } = &b[1]
    else {
        unreachable!()
    };
    // 픽스처 suffixes {"", filled} → 기본 칩 ""에서 home-filled는 빠진다 (evil·home·small-20-solid)
    assert_eq!((cells.len(), *total), (3, 3));
    assert!(serde_json::to_value(&b[1]).unwrap().get("more").is_none());
    assert_eq!((option.as_str(), selected), ("selected", &None));
    let chips = chips.as_ref().unwrap();
    assert_eq!((chips.option.as_str(), chips.value.as_str()), ("style", ""));
    assert_eq!(chips.choices.len(), 2);
    assert!(chips.resets.is_empty());
    let search = search.as_ref().unwrap();
    assert_eq!(
        (search.option.as_str(), search.placeholder.as_str()),
        ("query", "아이콘 검색")
    );
    assert!(search.resets.is_empty());
    assert_eq!(cells[0].key, cells[0].title);
    assert!(cells.iter().all(|c| c.svg.contains(r#"height="24""#)));

    let (_tmp, opts) = with_fixture(json!({ "style": "filled" }));
    let b = run(opts);
    let Block::IconGrid { cells, total, .. } = &b[1] else {
        unreachable!()
    };
    assert_eq!((cells.len(), *total), (1, 1));
    assert_eq!(cells[0].key, "home-filled");
}

#[test]
fn unsuffixed_icons_show_in_every_chip() {
    let icons: Vec<Icon> = ["bold", "home-line", "home-fill"].map(icon).into();
    let ri = list(&[("line", "선"), ("fill", "채움")]);
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
    let os = list(&[("outline", "외곽선")]);
    assert_eq!(styles::filter(&one, "", Some("outline"), &os).len(), 2);
    assert_eq!(styles::filter(&one, "", None, &os).len(), 2);
}

#[test]
fn styles_of_and_style_of_match_v1() {
    let hero = list(&[
        ("", "선 24"),
        ("solid", "채움 24"),
        ("20-solid", "채움 20"),
        ("16-solid", "채움 16"),
    ]);
    assert_eq!(styles::style_of("academic-cap-20-solid", &hero), "20-solid");
    assert_eq!(styles::style_of("academic-cap-solid", &hero), "solid");
    assert_eq!(styles::style_of("academic-cap", &hero), "");
    let tb = styles::styles_of(&test_set("tabler"));
    assert_eq!(tb, list(&[("", "선"), ("filled", "채움")]));
    assert_eq!(styles::style_of("home-filled", &tb), "filled");
    // 1판 추천 세트는 1판 이름·순서 (metadata는 무시)
    let mut ri = test_set("ri");
    ri.suffixes = map(&[("fill", "Fill"), ("line", "Line")]);
    assert_eq!(
        styles::styles_of(&ri),
        list(&[("line", "선"), ("fill", "채움")])
    );
    assert_eq!(styles::styles_of(&test_set("heroicons")), hero);
    // 그 밖: metadata suffixes, "" 먼저 나머지는 키 순서
    let mut other = test_set("other");
    assert!(styles::styles_of(&other).is_empty());
    other.suffixes = map(&[("solid", "Solid"), ("", "Outline"), ("bold", "Bold")]);
    assert_eq!(
        styles::styles_of(&other),
        list(&[("", "Outline"), ("bold", "Bold"), ("solid", "Solid")])
    );
}

#[test]
fn ri_default_chip_is_line_in_v1_order() {
    let tmp = layout(
        "ri",
        "1.0.0",
        Some(
            r#"{"icons":{"bold":{"body":"<path d=\"M0 0\"/>"},"home-fill":{"body":"<path d=\"M0 0\"/>"},"home-line":{"body":"<path d=\"M0 0\"/>"}}}"#,
        ),
    );
    let b = run(json!({
        "set": "ri",
        "__resources": { "dir": tmp.path(), "installed": { "ri": "1.0.0" } }
    }));
    let Block::IconGrid { cells, chips, .. } = &b[1] else {
        unreachable!()
    };
    let chips = chips.as_ref().unwrap();
    assert_eq!(chips.value, "line");
    let choices: Vec<(&str, &str)> = chips
        .choices
        .iter()
        .map(|c| (c.value.as_str(), c.label.as_str()))
        .collect();
    assert_eq!(choices, [("line", "선"), ("fill", "채움")]);
    let keys: Vec<&str> = cells.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["bold", "home-line"]);
}

#[test]
fn selected_produces_panel_blocks() {
    let (_tmp, opts) = with_fixture(json!({ "selected": "home" }));
    let b = run(opts);
    assert_eq!(
        types(&b),
        [
            "resourceList",
            "iconGrid",
            "image",
            "optionHints",
            "actions"
        ]
    );
    assert!(matches!(&b[1], Block::IconGrid { selected: Some(s), .. } if s == "home"));
    assert_eq!(
        labels(&b),
        ["SVG 복사", "PNG 복사", "Data URI 복사", "SVG 저장"]
    );
    let kinds: Vec<_> = serde_json::to_value(&b[4]).unwrap()["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(kinds, ["copyText", "copyImage", "copyText", "saveText"]);
    let Action::SaveText { name, .. } = &actions(&b)[3].action else {
        unreachable!()
    };
    assert_eq!(name, "home.svg");
    let Action::CopyImage { src } = &actions(&b)[1].action else {
        unreachable!()
    };
    assert!(src.starts_with("data:image/png;base64,"));
    let svg = text_of(&b, "SVG 복사");
    assert!(
        svg.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">"#
        ),
        "{svg}"
    );
    // 기본 색은 #000000
    assert!(svg.contains(r#"stroke-width="2""#) && svg.contains("#000000"));
    assert!(!svg.contains("currentColor"), "{svg}");
    assert!(text_of(&b, "Data URI 복사").starts_with(r#"url("data:image/svg+xml,%3Csvg"#));

    // 크기·두께·색 옵션
    let (_tmp, opts) = with_fixture(json!({
        "selected": "home", "size": 48, "stroke": 1.5, "color": "#ff0000"
    }));
    let b = run(opts);
    let svg = text_of(&b, "SVG 복사");
    assert!(
        svg.contains(r#"width="48" height="48""#) && svg.contains(r#"stroke-width="1.5""#),
        "{svg}"
    );
    assert!(
        svg.contains("#ff0000") && !svg.contains("currentColor"),
        "{svg}"
    );
}

#[test]
fn bad_color_is_reported_not_injected() {
    let (_tmp, opts) = with_fixture(json!({
        "selected": "home", "color": "red\"/><script>"
    }));
    let b = run(opts);
    assert!(actions(&b).is_empty());
    assert!(
        b.iter()
            .any(|b| matches!(b, Block::Text { text } if text.contains("#rrggbb")))
    );
}

#[test]
fn unknown_selected_gives_no_panel_blocks() {
    let (_tmp, opts) = with_fixture(json!({ "selected": "nope" }));
    let b = run(opts);
    assert_eq!(types(&b), ["resourceList", "iconGrid"]);
    assert!(matches!(&b[1], Block::IconGrid { selected: None, .. }));
}

#[test]
fn filtered_out_selection_keeps_panel_blocks() {
    // home-filled는 기본 칩("")에서, home은 검색어로 격자에서 가려진다
    for opts in [
        json!({ "selected": "home-filled" }),
        json!({ "selected": "home", "query": "evil" }),
    ] {
        let sel = opts["selected"].as_str().unwrap().to_string();
        let (_tmp, opts) = with_fixture(opts);
        let b = run(opts);
        // home-filled는 선 두께가 없어 optionHints가 없다
        let t: Vec<_> = types(&b)
            .into_iter()
            .filter(|t| t != "optionHints")
            .collect();
        assert_eq!(t, ["resourceList", "iconGrid", "image", "actions"], "{sel}");
        let Block::IconGrid { cells, .. } = &b[1] else {
            unreachable!()
        };
        assert!(!cells.iter().any(|c| c.key == sel), "{sel}");
        assert!(
            matches!(&actions(&b).last().unwrap().action, Action::SaveText { name, .. } if *name == format!("{sel}.svg"))
        );
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
        render::data_uri("<svg/>"),
        r#"url("data:image/svg+xml,%3Csvg%2F%3E")"#
    );
    assert_eq!(
        render::data_uri("a b#'()*~\u{e9}"),
        "url(\"data:image/svg+xml,a%20b%23'()*~%C3%A9\")"
    );
}

/// PNG 복사 버튼 그림의 (가로, 세로)
fn png_size(b: &[Block]) -> (u32, u32) {
    let Action::CopyImage { src } = &actions(b)[1].action else {
        unreachable!()
    };
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        src.strip_prefix("data:image/png;base64,").unwrap(),
    )
    .unwrap();
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn png_size_scales() {
    // size 옵션 48 → PNG 48x48 (1배)
    let (_tmp, opts) = with_fixture(json!({ "selected": "home", "size": 48 }));
    assert_eq!(png_size(&run(opts)), (48, 48));
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

/// 패널 미리보기(`image`, 이름 없음)의 SVG
fn preview_svg(blocks: &[Block]) -> String {
    let src = blocks
        .iter()
        .find_map(|b| match b {
            Block::Image { src, name } if name.is_empty() => Some(src.clone()),
            _ => None,
        })
        .expect("미리보기");
    let b64 = src.strip_prefix("data:image/svg+xml;base64,").unwrap();
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn preview_image_matches_copied_svg() {
    for opts in [
        json!({ "selected": "home" }),
        json!({ "selected": "home", "size": 200, "stroke": 1.25, "color": "#2563EB" }),
    ] {
        let (_tmp, opts) = with_fixture(opts);
        let b = run(opts);
        assert_eq!(preview_svg(&b), text_of(&b, "SVG 복사"));
    }
    let (_tmp, opts) = with_fixture(json!({ "selected": "home", "size": 200 }));
    assert!(preview_svg(&run(opts)).contains(r#"width="200" height="200""#));
}

fn hints(blocks: &[Block]) -> Option<BTreeMap<String, Value>> {
    blocks.iter().find_map(|b| match b {
        Block::OptionHints { values } => Some(values.clone()),
        _ => None,
    })
}

#[test]
fn option_hints_carry_original_stroke_and_size() {
    let (_tmp, opts) = with_fixture(json!({ "selected": "home" }));
    let h = hints(&run(opts)).unwrap();
    assert_eq!(h.get("stroke").and_then(Value::as_f64), Some(2.0));
    assert_eq!(h.get("size").and_then(Value::as_u64), Some(24));
    // 사용자가 두께·크기를 정해도 힌트는 원래 값
    let (_tmp, opts) = with_fixture(json!({ "selected": "home", "stroke": 1.5, "size": 48 }));
    let h = hints(&run(opts)).unwrap();
    assert_eq!(h.get("stroke").and_then(Value::as_f64), Some(2.0));
    assert_eq!(h.get("size").and_then(Value::as_u64), Some(24));
    // 선 두께가 없는 아이콘은 크기 힌트만
    let (_tmp, opts) = with_fixture(json!({ "selected": "home-filled" }));
    let h = hints(&run(opts)).unwrap();
    assert!(h.get("stroke").is_none());
    assert_eq!(h.get("size").and_then(Value::as_u64), Some(24));
}

#[test]
fn null_size_uses_icon_height() {
    // small-20-solid는 20×20: 크기를 비우면 20으로, PNG도 20×20
    let (_tmp, opts) = with_fixture(json!({ "selected": "small-20-solid", "size": null }));
    let b = run(opts);
    assert_eq!(
        hints(&b).unwrap().get("size").and_then(Value::as_u64),
        Some(20)
    );
    let svg = text_of(&b, "SVG 복사");
    assert!(svg.contains(r#"width="20" height="20""#), "{svg}");
    assert_eq!(preview_svg(&b), svg);
    assert_eq!(png_size(&b), (20, 20));
    // 정하면 그 크기
    let (_tmp, opts) = with_fixture(json!({ "selected": "small-20-solid", "size": 40 }));
    let b = run(opts);
    assert!(text_of(&b, "SVG 복사").contains(r#"width="40" height="40""#));
    assert_eq!(png_size(&b), (40, 40));
}

#[test]
fn stroke_width_reads_first_value() {
    assert_eq!(
        render::stroke_width(r#"<g stroke-width="1.5"><path stroke-width="2"/></g>"#),
        Some(1.5)
    );
    assert_eq!(render::stroke_width(r#"<path d="M0 0"/>"#), None);
    assert_eq!(render::stroke_width(r#"<path stroke-width="x"/>"#), None);
}

#[test]
fn palette_set_ignores_color() {
    let tmp = layout("palette-set", "1.0.0", None);
    std::fs::write(
        tmp.path().join("palette-set/1.0.0/info.json"),
        r#"{ "name": "Color", "total": 5, "license": { "title": "MIT", "spdx": "MIT" }, "palette": true }"#,
    )
    .unwrap();
    let b = run(json!({
        "set": "palette-set", "selected": "home", "color": "#ff0000",
        "__resources": { "dir": tmp.path(), "installed": { "palette-set": "1.0.0" } }
    }));
    let svg = text_of(&b, "SVG 복사");
    assert!(
        !svg.contains("#ff0000") && svg.contains("currentColor"),
        "{svg}"
    );
}

#[test]
fn png_failure_keeps_other_buttons() {
    // 가로로 아주 긴 아이콘: 512px 높이면 PNG 픽셀 한도를 넘는다
    let tmp = layout(
        "wide-set",
        "1.0.0",
        Some(r#"{"icons":{"wide":{"body":"<path d=\"M0 0h1\"/>","width":100000,"height":1}}}"#),
    );
    let b = run(json!({
        "set": "wide-set", "selected": "wide", "size": 512,
        "__resources": { "dir": tmp.path(), "installed": { "wide-set": "1.0.0" } }
    }));
    assert_eq!(labels(&b), ["SVG 복사", "Data URI 복사", "SVG 저장"]);
}
