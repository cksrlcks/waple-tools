use std::path::{Path, PathBuf};

use resvg::tiny_skia::Pixmap;
use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;
use crate::numbers::{round_numbers, round_path};

/// Inkscape가 저장한 것 같은 SVG: 주석·metadata·편집기 속성·빈 그룹·긴 소수
const INKSCAPE: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<!-- Created with Inkscape -->
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
  xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:dc="http://purl.org/dc/elements/1.1/"
  xmlns:cc="http://creativecommons.org/ns#"
  width="64" height="48" inkscape:version="1.3" sodipodi:docname="t.svg">
  <sodipodi:namedview id="nv" inkscape:zoom="2"/>
  <metadata><rdf:RDF><cc:Work><dc:title>t</dc:title></cc:Work></rdf:RDF></metadata>
  <defs>
    <linearGradient id="lg" x1="0" y1="0" x2="1.000001" y2="0">
      <stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/>
    </linearGradient>
  </defs>
  <g inkscape:label="Layer 1" inkscape:groupmode="layer">
    <g><g/></g>
    <defs/>
    <rect x="2.00004" y="3.123456" width="20.5" height="10.25" fill="url(#lg)"/>
    <path d="m 30.123456,5.987654 l 10.111111,0.222222 l 0.333333,10.444444 l -10.555555,0.111111 z
             m 2,2 h 3.3333 v 3.3333 h -3.3333 z" fill="#0a0" fill-rule="evenodd"/>
    <circle cx="12.3456789" cy="34.5678" r="8.7654321" fill="none" stroke="#333" stroke-width="2.5"/>
    <use xlink:href="#dot" transform="translate(40.00001 30.99999)"/>
    <path id="dot" d="M0 0a4.5 4.5 0 1 0 9 0a4.5 4.5 0 1 0 -9 0" fill="#c0c"/>
  </g>
  <!--! 저작권 -->
</svg>"##;

fn square_svg() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/square.svg"))
        .unwrap()
}

/// `name` 파일로 두고 실행한 (결과 이름, 내용). 원본이 그대로인지도 본다
fn run(name: &str, bytes: &[u8], opts: serde_json::Value) -> Result<(PathBuf, Vec<u8>), ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join(name);
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let res = tool().run(&ctx, std::slice::from_ref(&src), &out, &opts);
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    let ToolOutput::Files(names) = res? else {
        panic!("파일이어야 함");
    };
    let data = std::fs::read(out.join(&names[0])).unwrap();
    Ok((names[0].clone(), data))
}

fn optimized(svg: &str, opts: serde_json::Value) -> String {
    let (name, out) = run("t.svg", svg.as_bytes(), opts).unwrap();
    assert_eq!(name, PathBuf::from("t.svg"));
    String::from_utf8(out).unwrap()
}

fn png(svg: &[u8], opts: serde_json::Value) -> Pixmap {
    let (name, out) = run("t.svg", svg, opts).unwrap();
    assert_eq!(name, PathBuf::from("t.png"));
    Pixmap::decode_png(&out).unwrap()
}

#[test]
fn optimize_strips_editor_data_and_keeps_view_box() {
    let out = optimized(INKSCAPE, json!({}));
    for gone in [
        "<?xml",
        "Created with",
        "inkscape",
        "sodipodi",
        "metadata",
        "rdf",
        "xmlns:dc",
        "xmlns:cc",
        "<defs/>",
        "\n",
    ] {
        assert!(!out.contains(gone), "{gone} 남음: {out}");
    }
    assert!(
        out.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#),
        "{out}"
    );
    assert!(out.contains(r#"viewBox="0 0 64 48""#), "{out}");
    assert!(out.contains("<!--! 저작권 -->"), "{out}");
    assert!(
        out.contains(r##"xmlns:xlink="http://www.w3.org/1999/xlink""##),
        "{out}"
    );
    assert!(out.contains(r##"xlink:href="#dot""##), "{out}");
    // 내용 있는 레이어 그룹 하나만 남는다
    assert_eq!(out.matches("<g").count(), 1, "{out}");
    assert!(out.contains("<linearGradient"), "{out}");
    assert!(
        out.len() < INKSCAPE.len() / 2,
        "{} → {}",
        INKSCAPE.len(),
        out.len()
    );
}

#[test]
fn optimize_rounds_numbers() {
    let out = optimized(INKSCAPE, json!({ "precision": 2 }));
    assert!(out.contains(r#"x="2" y="3.12""#), "{out}");
    assert!(out.contains(r#"cx="12.35" cy="34.57" r="8.77""#), "{out}");
    assert!(out.contains(r#"x2="1""#), "{out}");
    assert!(out.contains(r#"transform="translate(40 31)""#), "{out}");
    assert!(out.contains(r#"d="m30.12 5.99l10.11.22"#), "{out}");
    assert!(!out.contains("30.123"), "{out}");
}

#[test]
fn optimized_renders_the_same() {
    let opts = json!({ "mode": "png", "scale": 4 });
    let before = png(INKSCAPE.as_bytes(), opts.clone());
    let out = optimized(INKSCAPE, json!({}));
    let after = png(out.as_bytes(), opts);
    assert_eq!((after.width(), after.height()), (256, 192));
    let diffs: Vec<u8> = before
        .data()
        .iter()
        .zip(after.data())
        .map(|(a, b)| a.abs_diff(*b))
        .collect();
    let max = diffs.iter().max().unwrap();
    let changed = diffs.iter().filter(|&&d| d > 2).count();
    // 가장자리 안티앨리어싱이 하위 샘플 경계를 넘으면 1/16(16) 단위로 달라진다
    assert!(
        *max <= 32 && changed * 1000 < diffs.len(),
        "최대 {max}, 달라진 값 {changed}"
    );
    // 실제로 그린 게 있어야 비교가 의미 있다
    assert!(before.data().chunks(4).filter(|p| p[3] > 0).count() > 1000);
}

#[test]
fn options_can_keep_comments_and_groups() {
    let out = optimized(
        INKSCAPE,
        json!({ "stripMetadata": false, "removeEmpty": false }),
    );
    assert!(out.contains("<!-- Created with Inkscape -->"), "{out}");
    assert!(out.contains("<sodipodi:namedview"), "{out}");
    assert!(out.contains(r#"inkscape:label="Layer 1""#), "{out}");
    assert!(out.contains("xmlns:inkscape="), "{out}");
    assert!(out.contains("<g><g/></g>"), "{out}");
    assert!(!out.contains("<?xml"), "{out}");
}

#[test]
fn empty_but_visible_containers_stay() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"><mask id="m"/><g filter="url(#f)"/><pattern id="p" href="#q"/><g/></svg>"##;
    let out = optimized(svg, json!({}));
    assert_eq!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"><mask id="m"/><g filter="url(#f)"/><pattern id="p" href="#q"/></svg>"##
    );
}

#[test]
fn doctype_entities_style_and_text_survive() {
    let svg = r#"<?xml version="1.0"?>
<!DOCTYPE svg [ <!ENTITY ns_svg "http://www.w3.org/2000/svg"> ]>
<svg xmlns="&ns_svg;" width="10px" height="5">
  <style><![CDATA[ a < b { fill: red } ]]></style>
  <text x="1" y="4"><tspan>a</tspan> <tspan>b &amp; c</tspan></text>
</svg>"#;
    let out = optimized(svg, json!({}));
    assert_eq!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10px" height="5" viewBox="0 0 10 5"><style><![CDATA[ a < b { fill: red } ]]></style><text x="1" y="4"><tspan>a</tspan> <tspan>b &amp; c</tspan></text></svg>"#
    );
}

#[test]
fn path_rounding_does_not_drift() {
    // 1.4씩 10번 = 14. 자릿수 0이어도 상대 좌표 오차가 쌓이지 않는다 (따로 반올림하면 10)
    let d = format!("M0 0{}", " l1.4 0".repeat(10));
    let out = round_path(&d, 0).unwrap();
    let end: f64 = svgtypes::PathParser::from(out.as_str())
        .filter_map(|s| match s.unwrap() {
            svgtypes::PathSegment::LineTo { x, .. } => Some(x),
            _ => None,
        })
        .sum();
    assert_eq!(end, 14.0, "{out}");
    // 호의 플래그는 붙여 써도 그대로, 읽을 수 없는 path는 None
    assert_eq!(
        round_path("M0 0a1 1 0 01.5.5", 1).unwrap(),
        "M0 0a1 1 0 0 1 .5.5"
    );
    assert_eq!(round_path("M0 0 L", 1), None);
}

#[test]
fn number_lists_round_safely() {
    assert_eq!(round_numbers("1.5.5", 0, 0), "2 1");
    // 0이 아니던 값은 0으로 만들지 않는다
    assert_eq!(
        round_numbers("12.345% 10 007 1e-7 -0.0004", 1, 0),
        "12.3% 10 007 .0000001 -.0004"
    );
    assert_eq!(
        round_numbers("matrix(1.00001,0,0,1,-0.33333,2.5)", 2, 0),
        "matrix(1,0,0,1,-.33,2.5)"
    );
    // transform은 유효숫자 5개, 큰 이동값은 자릿수 기준
    assert_eq!(
        round_numbers("scale(0.00048828125) translate(1234.56789 0.123456)", 2, 5),
        "scale(.00048828) translate(1234.57 .12346)"
    );
}

#[test]
fn tiny_values_are_not_rounded_away() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><g transform="scale(0.00048828125)"><rect width="2048" height="2048"/></g><path d="M0 0l.0004 0" stroke="#000" stroke-width="0.0004"/></svg>"##;
    let out = optimized(svg, json!({}));
    assert!(out.contains(r#"transform="scale(.00048828)""#), "{out}");
    assert!(out.contains(r#"d="M0 0l.0004 0""#), "{out}");
    assert!(out.contains(r#"stroke-width=".0004""#), "{out}");
    let p = png(
        out.as_bytes(),
        json!({ "mode": "png", "sizeMode": "width", "width": 8 }),
    );
    assert!(
        p.data().chunks(4).all(|px| px[3] > 0),
        "사각형이 그려져야 함"
    );
}

#[test]
fn close_path_letters_are_kept() {
    assert_eq!(round_path("M0 0L1 1ZZ", 1).unwrap(), "M0 0L1 1ZZ");
}

#[test]
fn svg_without_xmlns_is_still_svg() {
    let svg = r#"<svg width="10" height="5"><style>a > b { fill: red }</style><text x="1.23456" y="4"><tspan>a</tspan> <tspan>b</tspan></text><g/></svg>"#;
    assert_eq!(
        optimized(svg, json!({})),
        r#"<svg width="10" height="5" viewBox="0 0 10 5"><style>a > b { fill: red }</style><text x="1.235" y="4"><tspan>a</tspan> <tspan>b</tspan></text></svg>"#
    );
}

#[test]
fn entity_bombs_are_rejected_quickly() {
    let lol = format!(
        r#"<!DOCTYPE svg [ <!ENTITY a "{}"> <!ENTITY b "{}"> ]><svg xmlns="http://www.w3.org/2000/svg"><text>{}</text></svg>"#,
        "lol".repeat(400),
        "&a;".repeat(255),
        "&b;".repeat(300)
    );
    for mode in ["optimize", "png"] {
        let t = std::time::Instant::now();
        let e = run("x.svg", lol.as_bytes(), json!({ "mode": mode })).unwrap_err();
        assert!(e.to_string().contains("엔티티"), "{mode}: {e}");
        assert!(t.elapsed().as_secs() < 2, "{mode}: {:?}", t.elapsed());
    }
}

#[test]
fn huge_embedded_images_are_skipped() {
    let mut px = Pixmap::new(4, 4).unwrap();
    px.fill(resvg::tiny_skia::Color::BLACK);
    let png = px.encode_png().unwrap();
    assert!(render::data_ok(&png, 16));
    assert!(!render::data_ok(&png, 15));
}

#[test]
fn structural_css_and_referenced_ids_keep_elements() {
    // :nth-child가 있으면 빈 요소·metadata를 지우면 대상이 바뀐다
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>rect:nth-child(3){fill:red}</style><metadata/><g/><rect width="1" height="1"/></svg>"#;
    let out = optimized(svg, json!({}));
    assert!(out.contains("<metadata/><g/>"), "{out}");
    // #id로 가리키는 빈 그룹은 남긴다
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><g id="slot"/><g id="free"/><use href="#slot"/></svg>"##;
    let out = optimized(svg, json!({}));
    assert!(out.contains(r#"<g id="slot"/>"#), "{out}");
    assert!(!out.contains("free"), "{out}");
}

#[test]
fn png_sizes() {
    let sq = square_svg();
    let p = png(&sq, json!({ "mode": "png", "scale": 2 }));
    assert_eq!((p.width(), p.height()), (16, 16));
    assert_eq!(p.pixel(8, 8).unwrap().demultiply().red(), 0xee);

    let p = png(
        INKSCAPE.as_bytes(),
        json!({ "mode": "png", "sizeMode": "width", "width": 32 }),
    );
    assert_eq!((p.width(), p.height()), (32, 24));
}

#[test]
fn png_background() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect x="5" y="5" width="5" height="5" fill="#00f"/></svg>"##;
    let p = png(svg, json!({ "mode": "png" }));
    assert_eq!(p.pixel(0, 0).unwrap().alpha(), 0);
    let p = png(
        svg,
        json!({ "mode": "png", "background": "solid", "color": "#ff0000" }),
    );
    let c = p.pixel(0, 0).unwrap();
    assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (255, 0, 0, 255));
    let c = p.pixel(7, 7).unwrap();
    assert_eq!((c.red(), c.blue()), (0, 255));

    let e = run(
        "t.svg",
        svg,
        json!({ "mode": "png", "background": "solid", "color": "red" }),
    )
    .unwrap_err();
    assert!(e.to_string().contains("#rrggbb"), "{e}");
}

#[test]
fn png_ignores_external_images() {
    let dir = tempfile::tempdir().unwrap();
    let mut red = Pixmap::new(4, 4).unwrap();
    red.fill(resvg::tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    let ext = dir.path().join("red.png");
    red.save_png(&ext).unwrap();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><image href="{}" width="4" height="4"/></svg>"#,
        ext.display()
    );
    let p = png(svg.as_bytes(), json!({ "mode": "png" }));
    assert!(p.data().iter().all(|&b| b == 0), "외부 파일을 읽으면 안 됨");
}

#[test]
fn png_too_large_fails() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="20000" height="20000"/>"#;
    let e = run("t.svg", svg, json!({ "mode": "png", "scale": 2 })).unwrap_err();
    assert!(e.to_string().contains("1억 화소"), "{e}");
}

#[test]
fn bad_files_are_korean_errors() {
    for mode in ["optimize", "png"] {
        let opts = json!({ "mode": mode });
        for bad in [&b"<svg"[..], &[0xff, 0xfe, 0x00][..], b"hello"] {
            let e = run("x.svg", bad, opts.clone()).unwrap_err();
            assert!(e.to_string().contains("손상"), "{mode}: {e}");
        }
    }
    let e = run("x.svg", b"<html/>", json!({})).unwrap_err();
    assert!(e.to_string().contains("SVG 파일이 아닙니다"), "{e}");
    let e = run("x.svg", b"<html/>", json!({ "mode": "png" })).unwrap_err();
    assert!(matches!(e, ToolError::Failed(_)), "{e}");
}

#[test]
fn schema_and_meta() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "none");
    assert_eq!(
        s["x-allow-larger-when"],
        serde_json::json!({ "mode": ["png"] })
    );
    assert_eq!(
        s["properties"]["color"]["x-enabled-when"],
        json!({ "mode": ["png"], "background": ["solid"] })
    );
    assert_eq!(s["properties"]["precision"]["default"], 3);
    let m = tool().meta();
    assert_eq!(
        (m.id.as_str(), m.name.as_str(), m.category.as_str()),
        ("image.svg", "SVG 도구", "image")
    );
    assert_eq!(m.description, "SVG를 최적화하거나 PNG로 내보냅니다.");
    assert_eq!(m.extensions, ["svg"]);
}
