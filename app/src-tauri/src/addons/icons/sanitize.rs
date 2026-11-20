//! 아이콘 SVG 본문 정리. 본문은 웹뷰에 인라인 SVG로 들어가므로 허용 목록 밖은 모두 버린다.

const ELEMENTS: &[&str] = &[
    "g",
    "path",
    "circle",
    "rect",
    "line",
    "polyline",
    "polygon",
    "ellipse",
    "defs",
    "clipPath",
    "mask",
    "linearGradient",
    "radialGradient",
    "stop",
];

const ATTRS: &[&str] = &[
    "d",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-opacity",
    "fill-opacity",
    "fill-rule",
    "clip-rule",
    "opacity",
    "transform",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "width",
    "height",
    "points",
    "id",
    "clip-path",
    "mask",
    "offset",
    "stop-color",
    "stop-opacity",
    "gradientUnits",
    "gradientTransform",
    "fx",
    "fy",
    "pathLength",
];

const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// `url(`은 `url(#`로 시작하는 참조만 허용한다 (외부 주소 차단).
fn value_ok(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    lower
        .match_indices("url(")
        .all(|(i, _)| lower[i + 4..].trim_start().starts_with('#'))
}

fn escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write_node(n: roxmltree::Node, out: &mut String) {
    // 텍스트·주석·처리 명령은 버린다.
    if !n.is_element() {
        return;
    }
    let t = n.tag_name();
    if t.namespace() != Some(SVG_NS) || !ELEMENTS.contains(&t.name()) {
        return;
    }
    out.push('<');
    out.push_str(t.name());
    for a in n.attributes() {
        // 네임스페이스가 붙은 속성(xlink:href 등)은 이름이 같아 보여도 버린다.
        if a.namespace().is_none() && ATTRS.contains(&a.name()) && value_ok(a.value()) {
            out.push_str(&format!(" {}=\"{}\"", a.name(), escape(a.value())));
        }
    }
    if n.children().any(|c| c.is_element()) {
        out.push('>');
        for c in n.children() {
            write_node(c, out);
        }
        out.push_str(&format!("</{}>", t.name()));
    } else {
        out.push_str("/>");
    }
}

/// 정리된 본문을 돌려준다. XML로 읽을 수 없으면 None.
pub fn sanitize_body(body: &str) -> Option<String> {
    let xml = format!("<svg xmlns=\"{SVG_NS}\">{body}</svg>");
    let doc = roxmltree::Document::parse(&xml).ok()?;
    let mut out = String::new();
    for c in doc.root_element().children() {
        write_node(c, &mut out);
    }
    Some(out)
}
