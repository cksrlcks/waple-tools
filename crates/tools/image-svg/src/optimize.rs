//! SVG 최적화: SVGO 기본 동작 중 렌더링을 바꾸지 않는 것만 한다.
//! XML 선언·DOCTYPE(엔티티는 풀어 쓴다)·주석·`<metadata>`·편집기 데이터·빈 컨테이너를 지우고,
//! 좌표 숫자를 반올림하고, 요소 사이 공백을 지운다. viewBox는 지우지 않고 없으면 만든다.

use std::collections::{HashMap, HashSet};

use resvg::usvg::roxmltree::{Document, Node, NodeType, ParsingOptions};
use waple_tools_core::ToolError;

use crate::numbers::{round_numbers, round_path};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// 편집기 전용 네임스페이스 (SVGO removeEditorsNSData 목록). 이 안의 요소·속성은 화면에 영향이 없다
const EDITOR_NS: &[&str] = &[
    "http://creativecommons.org/ns#",
    "http://inkscape.sourceforge.net/DTD/s odipodi-0.dtd",
    "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://ns.adobe.com/AdobeIllustrator/10.0/",
    "http://ns.adobe.com/AdobeSVGViewerExtensions/3.0/",
    "http://ns.adobe.com/Extensibility/1.0/",
    "http://ns.adobe.com/Flows/1.0/",
    "http://ns.adobe.com/GenericCustomNamespace/1.0/",
    "http://ns.adobe.com/Graphs/1.0/",
    "http://ns.adobe.com/ImageReplacement/1.0/",
    "http://ns.adobe.com/SaveForWeb/1.0/",
    "http://ns.adobe.com/Variables/1.0/",
    "http://ns.adobe.com/XPath/1.0/",
    "http://purl.org/dc/elements/1.1/",
    "http://schemas.microsoft.com/visio/2003/SVGExtensions/",
    "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://taptrix.com/vectorillustrator/svg_extensions",
    "http://www.bohemiancoding.com/sketch/ns",
    "http://www.figma.com/figma/ns",
    "http://www.inkscape.org/namespaces/inkscape",
    "http://www.serif.com/",
    "http://www.vector.evaxdesign.sk",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
];

/// 좌표·크기 숫자 속성 (반올림 대상)
const NUMERIC: &[&str] = &[
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "fx",
    "fy",
    "width",
    "height",
    "points",
    "transform",
    "gradientTransform",
    "patternTransform",
    "viewBox",
    "stroke-width",
];

/// 안의 공백이 글자로 보이거나 의미가 있는 요소
const TEXT_CTX: &[&str] = &["text", "foreignObject", "style", "script", "title", "desc"];

pub struct Clean {
    pub precision: u8,
    pub strip_meta: bool,
    pub remove_empty: bool,
}

pub fn broken() -> ToolError {
    ToolError::Failed("SVG 파일이 손상되었거나 읽을 수 없습니다.".into())
}

/// 정리한 SVG 텍스트
pub fn optimize(bytes: &[u8], c: &Clean) -> Result<String, ToolError> {
    let text = std::str::from_utf8(bytes).map_err(|_| broken())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let opt = ParsingOptions {
        // Illustrator SVG는 DOCTYPE 안 엔티티로 네임스페이스를 적는다. 엔티티 폭탄은 roxmltree가 막는다
        allow_dtd: true,
        ..Default::default()
    };
    check_entities(text)?;
    let doc = Document::parse_with_options(text, opt).map_err(|_| broken())?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(ToolError::Failed("SVG 파일이 아닙니다.".into()));
    }
    let styles: String = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "style")
        .filter_map(|n| n.text())
        .collect();
    let mut w = Writer {
        c,
        used: HashSet::new(),
        // HTML에 인라인하려고 xmlns를 뺀 SVG: 네임스페이스 없는 요소도 SVG로 본다 (usvg와 같다)
        bare: root.tag_name().namespace().is_none(),
        // 형제 순서·빈 요소를 보는 선택자가 있으면 요소를 지우거나 공백을 빼면 대상이 바뀐다
        keep_structure: STRUCTURAL.iter().any(|k| styles.contains(k)),
        refs: referenced_ids(text),
    };
    let mut out = String::with_capacity(text.len());
    for n in doc.root().children() {
        w.node(n, false, &mut out);
    }
    Ok(out)
}

/// 구조를 보는 CSS 선택자 (`+`·`~`는 형제 결합자). 값 안의 `+`도 걸리지만 안전한 쪽으로 둔다
const STRUCTURAL: &[&str] = &[":nth-", ":first-", ":last-", ":only-", ":empty", "+", "~"];

/// 문서에서 `#이름`으로 가리킨 이름 (href, url(#…), CSS). 색 #fff 같은 것도 들어가지만 지우지 않는 쪽이라 괜찮다
fn referenced_ids(text: &str) -> HashSet<&str> {
    text.split('#')
        .skip(1)
        .map(|s| {
            let end = s
                .find(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')))
                .unwrap_or(s.len());
            &s[..end]
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// DOCTYPE 엔티티를 풀어 늘어날 크기 상한. roxmltree는 최상위 참조 개수를 세지 않아
/// 큰 엔티티를 수없이 참조하면 수 GB로 불어난다 (앱과 같은 프로세스라 메모리 부족으로 죽는다)
const MAX_ENTITY_BYTES: u64 = 64 << 20;

/// 엔티티 선언과 참조로 풀린 크기를 파싱 전에 어림해 상한을 넘으면 거부한다
pub fn check_entities(text: &str) -> Result<(), ToolError> {
    if !text.contains("<!ENTITY") {
        return Ok(());
    }
    let mut defs = HashMap::new();
    for decl in text.split("<!ENTITY").skip(1) {
        let decl = decl.trim_start();
        let decl = decl.strip_prefix('%').unwrap_or(decl).trim_start();
        let Some((name, rest)) = decl.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim_start();
        // SYSTEM·PUBLIC 외부 엔티티는 읽지 않는다
        let Some(q) = rest.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
            continue;
        };
        if let Some((value, _)) = rest[1..].split_once(q) {
            defs.insert(name, value);
        }
    }
    let mut memo = HashMap::new();
    let mut total = 0u64;
    for name in entity_refs(text) {
        total = total.saturating_add(expanded(name, &defs, &mut memo, 0));
        if total > MAX_ENTITY_BYTES {
            return Err(ToolError::Failed(
                "SVG의 DOCTYPE 엔티티를 풀면 너무 커집니다 (64MB 초과).".into(),
            ));
        }
    }
    Ok(())
}

/// `&이름;` 참조들 (문자 참조 `&#…;`는 뺀다)
fn entity_refs(s: &str) -> impl Iterator<Item = &str> {
    s.split('&').skip(1).filter_map(|r| {
        let (name, _) = r.split_once(';')?;
        (!name.is_empty() && !name.starts_with('#') && !name.contains(char::is_whitespace))
            .then_some(name)
    })
}

/// 엔티티 하나를 끝까지 풀었을 때의 길이 (상한을 넘으면 거기서 멈춘다)
fn expanded<'a>(
    name: &'a str,
    defs: &HashMap<&'a str, &'a str>,
    memo: &mut HashMap<&'a str, u64>,
    depth: u8,
) -> u64 {
    if let Some(&n) = memo.get(name) {
        return n;
    }
    // 미리 정의된 &amp; 등은 0. 순환 참조는 roxmltree가 오류로 거부한다
    let (Some(value), true) = (defs.get(name), depth <= 16) else {
        return 0;
    };
    let mut n = value.len() as u64;
    for r in entity_refs(value) {
        n = n.saturating_add(expanded(r, defs, memo, depth + 1));
        if n > MAX_ENTITY_BYTES {
            break;
        }
    }
    memo.insert(name, n);
    n
}

struct Writer<'c, 't> {
    c: &'c Clean,
    /// 쓴 요소·속성이 쓰는 네임스페이스 URI. 안 쓰는 xmlns:접두사 선언을 지우는 데 쓴다
    used: HashSet<String>,
    /// 루트에 xmlns가 없다: 네임스페이스 없는 요소도 SVG
    bare: bool,
    /// 요소·공백을 지우지 않는다 (구조 선택자가 있는 style)
    keep_structure: bool,
    refs: HashSet<&'t str>,
}

fn is_editor(uri: &str) -> bool {
    EDITOR_NS.contains(&uri)
}

impl Writer<'_, '_> {
    fn svg(&self, n: Node) -> bool {
        match n.tag_name().namespace() {
            Some(ns) => ns == SVG_NS,
            None => self.bare,
        }
    }

    fn is_svg(&self, n: Node, name: &str) -> bool {
        self.svg(n) && n.tag_name().name() == name
    }

    fn node(&mut self, n: Node, text_ctx: bool, out: &mut String) {
        match n.node_type() {
            NodeType::Element => self.element(n, text_ctx, out),
            NodeType::Text => {
                let t = n.text().unwrap_or_default();
                if !text_ctx && !self.keep_structure && t.trim().is_empty() {
                    return;
                }
                let raw = n
                    .parent_element()
                    .is_some_and(|p| self.is_svg(p, "style") || self.is_svg(p, "script"));
                // CSS·스크립트는 &lt; 대신 CDATA로 둬야 HTML에 인라인해도 깨지지 않는다
                if raw && t.contains(['<', '&']) {
                    out.push_str("<![CDATA[");
                    out.push_str(&t.replace("]]>", "]]]]><![CDATA[>"));
                    out.push_str("]]>");
                } else if raw {
                    out.push_str(t);
                } else {
                    escape(out, t, false);
                }
            }
            NodeType::Comment => {
                let t = n.text().unwrap_or_default();
                // `<!--!`는 남기라는 저작권 주석 (SVGO와 같다)
                if !self.c.strip_meta || t.starts_with('!') {
                    out.push_str("<!--");
                    out.push_str(t);
                    out.push_str("-->");
                }
            }
            NodeType::PI => {
                if let Some(pi) = n.pi() {
                    out.push_str("<?");
                    out.push_str(pi.target);
                    if let Some(v) = pi.value {
                        out.push(' ');
                        out.push_str(v);
                    }
                    out.push_str("?>");
                }
            }
            NodeType::Root => {}
        }
    }

    fn element(&mut self, n: Node, text_ctx: bool, out: &mut String) {
        let tag = n.tag_name();
        let svg = self.svg(n);
        if self.c.strip_meta
            && !self.keep_structure
            && (tag.namespace().is_some_and(is_editor) || self.is_svg(n, "metadata"))
        {
            return;
        }
        let text_ctx = text_ctx || (svg && TEXT_CTX.contains(&tag.name()));

        // 자식을 먼저 써야 비었는지, 어떤 네임스페이스를 쓰는지 안다
        let mut body = String::new();
        for child in n.children() {
            self.node(child, text_ctx, &mut body);
        }
        if self.c.remove_empty && !self.keep_structure && body.is_empty() && self.removable(n) {
            return;
        }

        let mut attrs = String::new();
        for a in n.attributes() {
            if let Some(uri) = a.namespace() {
                if self.c.strip_meta && is_editor(uri) {
                    continue;
                }
                self.used.insert(uri.into());
            }
            let v = a.value();
            // SVG 요소만 손댄다 (foreignObject 안 HTML 값은 그대로)
            let v = match (svg && a.namespace().is_none()).then_some(a.name()) {
                Some("d") => round_path(v, self.c.precision).unwrap_or_else(|| collapse(v)),
                Some(name) if NUMERIC.contains(&name) => {
                    // transform 계열은 작은 배율·회전이 0이 되지 않게 유효숫자 5개는 남긴다
                    let sig = if name.ends_with("ransform") { 5 } else { 0 };
                    round_numbers(&collapse(v), self.c.precision, sig)
                }
                Some(_) => collapse(v),
                None => v.into(),
            };
            push_attr(&mut attrs, &qname(n, a.namespace(), a.name(), true), &v);
        }
        if n.parent().is_some_and(|p| p.is_root()) && !n.has_attribute("viewBox") {
            // viewBox가 있어야 CSS로 크기를 바꿔도 비율대로 늘고 준다
            let w = n.attribute("width").and_then(length);
            let h = n.attribute("height").and_then(length);
            if let (Some(w), Some(h)) = (w, h) {
                let vb = round_numbers(&format!("0 0 {w} {h}"), self.c.precision, 0);
                push_attr(&mut attrs, "viewBox", &vb);
            }
        }
        if let Some(uri) = tag.namespace() {
            self.used.insert(uri.into());
        }

        out.push('<');
        let name = qname(n, tag.namespace(), tag.name(), false);
        out.push_str(&name);
        let inherited: Vec<_> = n
            .parent_element()
            .map(|p| p.namespaces().map(|ns| (ns.name(), ns.uri())).collect())
            .unwrap_or_default();
        for ns in n.namespaces() {
            if ns.uri() == XML_NS || inherited.contains(&(ns.name(), ns.uri())) {
                continue;
            }
            match ns.name() {
                None => push_attr(out, "xmlns", ns.uri()),
                Some(p) if self.used.contains(ns.uri()) => {
                    push_attr(out, &format!("xmlns:{p}"), ns.uri())
                }
                Some(_) => {}
            }
        }
        out.push_str(&attrs);
        if body.is_empty() {
            out.push_str("/>");
        } else {
            out.push('>');
            out.push_str(&body);
            out.push_str("</");
            out.push_str(&name);
            out.push('>');
        }
    }

    /// 비어 있으면 지워도 되는 컨테이너 (SVGO removeEmptyContainers 조건)
    fn removable(&self, n: Node) -> bool {
        // 다른 곳에서 #id로 가리키는 요소는 남긴다 (use·스크립트가 쓸 수 있다)
        if !self.svg(n)
            || n.parent_element().is_some_and(|p| self.is_svg(p, "switch"))
            || n.attribute("id").is_some_and(|id| self.refs.contains(id))
        {
            return false;
        }
        match n.tag_name().name() {
            // 필터(feFlood 등)는 빈 그룹에서도 그린다
            "g" => {
                !n.has_attribute("filter")
                    && !n.attribute("style").is_some_and(|s| s.contains("filter"))
            }
            // href로 다른 패턴을 물려받을 수 있다
            "pattern" => n.attributes().len() == 0,
            // 빈 마스크는 대상을 가린다
            "mask" => !n.has_attribute("id"),
            "a" | "defs" | "glyph" | "marker" | "missing-glyph" | "switch" | "symbol" => true,
            _ => false,
        }
    }
}

/// 접두사 붙은 이름. 접두사는 원래 문서에서 그 URI에 붙인 것
fn qname(n: Node, ns: Option<&str>, local: &str, attr: bool) -> String {
    let Some(uri) = ns else {
        return local.into();
    };
    if !attr && n.default_namespace() == Some(uri) {
        return local.into();
    }
    if uri == XML_NS {
        return format!("xml:{local}");
    }
    match n
        .namespaces()
        .find(|ns| ns.name().is_some() && ns.uri() == uri)
        .and_then(|ns| ns.name())
    {
        Some(p) => format!("{p}:{local}"),
        None => local.into(),
    }
}

/// width·height가 숫자(px)면 그 숫자
fn length(s: &str) -> Option<&str> {
    let t = s.trim();
    let t = t.strip_suffix("px").unwrap_or(t);
    t.parse::<f64>().ok().filter(|v| *v > 0.0).map(|_| t)
}

/// 줄바꿈·연속 공백을 한 칸으로
fn collapse(v: &str) -> String {
    v.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_attr(out: &mut String, name: &str, v: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape(out, v, true);
    out.push('"');
}

fn escape(out: &mut String, s: &str, attr: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' if !attr => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\n' if attr => out.push_str("&#10;"),
            '\r' if attr => out.push_str("&#13;"),
            '\t' if attr => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
}
