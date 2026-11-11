//! 웹폰트 서브셋 (추가 설치, Generator): 고른 글자만 남겨 WOFF2·WOFF·TTF와 `@font-face` CSS를 `<이름>-subset/` 폴더에 만든다.
//! 입력 형식은 파일 내용으로 판단한다. TrueType은 skeravar(hb-subset 포팅), CFF(OTF)는 allsorts로 자른다.

#[cfg(test)]
mod tests;
mod woff;

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use skrifa::{FontRef, MetadataProvider, Tag, attribute::Style, string::StringId};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

/// KS X 1001 완성형 한글 2,350자 (EUC-KR 0xB0A1–0xC8FE 순서)
const KSX1001: &str = include_str!("ksx1001.txt");
/// 텍스트 파일 상한: 글자 목록을 뽑는 데 이보다 큰 파일은 필요 없다
const MAX_TEXT_FILE: u64 = 20 * 1024 * 1024;

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-preview" = "none",
    // 결과는 세트 폴더(Generator)지만 줄이는 도구라 처리 전후 용량을 보여 준다
    "x-report-size" = true,
    "x-require-any" = ["woff2", "woff", "ttf"],
    "x-require-message" = "출력 형식을 하나 이상 고르세요."
))]
struct SubsetOptions {
    /// 완성형 한글 2,350자
    ///
    /// KS X 1001 완성형 한글입니다. 일상 글에 쓰는 한글 대부분이 들어 있습니다. 호환 자모(ㅋ, ㅠ 등)와 특수 기호는 직접 입력으로 넣어 주세요.
    #[schemars(extend("x-group" = "남길 글자"))]
    hangul: bool,
    /// 영문·숫자·기호
    ///
    /// 키보드로 치는 영문 대소문자, 숫자, 문장 부호(U+0020–U+007E)
    #[schemars(extend("x-group" = "남길 글자"))]
    latin: bool,
    /// 직접 입력
    #[schemars(extend("x-group" = "남길 글자"))]
    custom: bool,
    /// 넣을 글자
    ///
    /// 여기 적은 글자를 모두 남깁니다 (예: 완성형에 없는 '똠', '•', '©').
    #[schemars(extend("x-group" = "남길 글자", "x-wide" = true, "x-enabled-when" = { "custom": [true] }))]
    custom_text: String,
    /// 텍스트 파일에서 추출
    ///
    /// 파일에 나오는 글자만 남깁니다. HTML·JS 같은 소스 파일도 됩니다.
    #[schemars(extend("x-group" = "남길 글자"))]
    from_file: bool,
    /// 텍스트 파일
    ///
    /// UTF-8 텍스트 파일 (20MB까지)
    #[schemars(extend(
        "format" = "file",
        "x-file-extensions" = ["txt", "html", "htm", "md", "csv", "json", "js", "ts", "tsx", "jsx", "vue", "xml", "srt"],
        "x-group" = "남길 글자",
        "x-enabled-when" = { "fromFile": [true] }
    ))]
    text_file: String,

    /// WOFF2
    ///
    /// 요즘 브라우저 모두에서 쓰는 가장 작은 웹폰트
    #[schemars(extend(
        "x-group" = "출력 형식",
        "x-group-note" = "OTF(CFF) 폰트는 커닝·합자가 빠집니다. 가능하면 TTF를 넣어 주세요."
    ))]
    woff2: bool,
    /// WOFF
    ///
    /// 아주 오래된 브라우저용
    #[schemars(extend("x-group" = "출력 형식"))]
    woff: bool,
    /// TTF·OTF
    ///
    /// 압축하지 않은 원래 형식 (CFF 폰트는 OTF)
    #[schemars(extend("x-group" = "출력 형식"))]
    ttf: bool,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            hangul: true,
            latin: true,
            custom: false,
            custom_text: String::new(),
            from_file: false,
            text_file: String::new(),
            woff2: true,
            woff: false,
            ttf: false,
        }
    }
}

struct WebFontSubset;

pub fn tool() -> Box<dyn Tool> {
    Box::new(WebFontSubset)
}

waple_tools_addon_abi::export_tool!(tool);

fn failed(msg: &str) -> ToolError {
    ToolError::Failed(msg.into())
}

fn broken() -> ToolError {
    failed("폰트 파일이 손상되었거나 읽을 수 없습니다.")
}

/// 파일 앞 4바이트로 형식을 보고 sfnt(TTF·OTF)로 푼다 (확장자는 믿지 않는다)
fn to_sfnt(bytes: &[u8]) -> Result<Vec<u8>, ToolError> {
    match bytes.get(..4) {
        Some(b"wOF2") => wuff::decompress_woff2(bytes).map_err(|_| broken()),
        Some(b"wOFF") => wuff::decompress_woff1(bytes).map_err(|_| broken()),
        Some(b"OTTO") | Some([0, 1, 0, 0]) | Some(b"true") => Ok(bytes.to_vec()),
        Some(b"ttcf") => Err(failed(
            "폰트 모음(TTC)은 지원하지 않습니다. 폰트 하나씩 된 TTF·OTF를 넣어 주세요.",
        )),
        _ => Err(failed(
            "폰트 파일을 읽을 수 없습니다 (TTF·OTF·WOFF·WOFF2가 아닙니다).",
        )),
    }
}

/// UTF-8 텍스트 파일 (BOM 제거)
fn read_text_file(path: &str) -> Result<String, ToolError> {
    if path.is_empty() {
        return Err(failed("글자를 뽑을 텍스트 파일을 골라 주세요."));
    }
    let open_err = |_| failed("텍스트 파일을 열 수 없습니다.");
    if std::fs::metadata(path).map_err(open_err)?.len() > MAX_TEXT_FILE {
        return Err(failed("텍스트 파일이 너무 큽니다 (20MB까지)."));
    }
    let text = String::from_utf8(std::fs::read(path).map_err(open_err)?)
        .map_err(|_| failed("UTF-8 텍스트 파일만 읽을 수 있습니다."))?;
    Ok(match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_owned(),
        None => text,
    })
}

/// 고른 범위의 합집합. 공백은 늘 넣고 줄바꿈 같은 제어 문자는 뺀다
fn chars(o: &SubsetOptions) -> Result<BTreeSet<char>, ToolError> {
    let mut set = BTreeSet::new();
    if o.hangul {
        set.extend(KSX1001.chars());
    }
    if o.latin {
        set.extend(' '..='~');
    }
    if o.custom {
        set.extend(o.custom_text.chars());
    }
    if o.from_file {
        set.extend(read_text_file(&o.text_file)?.chars());
    }
    set.retain(|c| !c.is_control());
    if set.iter().all(|&c| c == ' ') {
        return Err(failed(
            "남길 글자가 없습니다. 글자 범위를 고르거나 글자를 입력해 주세요.",
        ));
    }
    set.insert(' ');
    Ok(set)
}

/// 고른 글자만 남긴 sfnt. 폰트에 없는 글자는 조용히 빠진다
fn subset(font: &FontRef, chars: &BTreeSet<char>) -> Result<Vec<u8>, ToolError> {
    let cmap = font.charmap();
    if !chars.iter().any(|&c| c != ' ' && cmap.map(c).is_some()) {
        return Err(failed("고른 글자가 이 폰트에 하나도 없습니다."));
    }
    let cff = font.table_data(Tag::new(b"CFF ")).is_some()
        || font.table_data(Tag::new(b"CFF2")).is_some();
    // skeravar는 깨진 폰트에서 expect로 패닉한다
    catch_unwind(AssertUnwindSafe(|| {
        if cff {
            subset_cff(font, chars)
        } else {
            subset_ttf(font, chars)
        }
    }))
    .unwrap_or_else(|_| Err(broken()))
}

/// TrueType: hb-subset 기본값 (GSUB·GPOS·힌팅 유지, kern·DSIG 등은 버림)
fn subset_ttf(font: &FontRef, chars: &BTreeSet<char>) -> Result<Vec<u8>, ToolError> {
    use skeravar::{DEFAULT_DROP_TABLES, DEFAULT_LAYOUT_FEATURES, IntSet, Plan, SubsetFlags};
    let unicodes: IntSet<u32> = chars.iter().map(|&c| u32::from(c)).collect();
    // hb-subset 기본(0–6)에 라이선스(13·14)를 더한다: 폰트 라이선스 문구가 남아야 한다
    let name_ids = [0, 1, 2, 3, 4, 5, 6, 13, 14]
        .into_iter()
        .map(skrifa::raw::types::NameId::new)
        .collect();
    let plan = Plan::new(
        &IntSet::empty(),
        &unicodes,
        font,
        SubsetFlags::default(),
        &DEFAULT_DROP_TABLES.iter().copied().collect(),
        &IntSet::all(),
        &DEFAULT_LAYOUT_FEATURES.iter().copied().collect(),
        &name_ids,
        &IntSet::all(),
        &None,
    );
    skeravar::subset_font(font, &plan).map_err(|_| broken())
}

/// CFF는 글리프 윤곽·cmap·기본 표만 남긴다 (allsorts는 GSUB·GPOS를 자르지 못해 뺀다)
fn subset_cff(font: &FontRef, chars: &BTreeSet<char>) -> Result<Vec<u8>, ToolError> {
    use allsorts::binary::read::ReadScope;
    use allsorts::font_data::FontData;
    use allsorts::subset::{CmapTarget, SubsetProfile};
    let cmap = font.charmap();
    let mut gids: Vec<u16> = chars
        .iter()
        .filter_map(|&c| cmap.map(c))
        .filter_map(|g| u16::try_from(g.to_u32()).ok())
        .filter(|&g| g != 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    gids.insert(0, 0); // .notdef가 맨 앞이어야 한다
    let data = ReadScope::new(font.data().as_bytes())
        .read::<FontData<'_>>()
        .map_err(|_| broken())?;
    let provider = data.table_provider(0).map_err(|_| broken())?;
    allsorts::subset::subset(
        &provider,
        &gids,
        &SubsetProfile::Minimal,
        CmapTarget::Unicode,
    )
    .map_err(|_| broken())
}

/// CSS 문자열 안에 넣을 수 있게 (제어 문자는 뺀다)
fn css_str(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
}

/// url() 안 파일 이름: `%`·`#`는 URL에서 뜻이 있어 퍼센트 인코딩한다
fn css_url(name: &str) -> String {
    css_str(&name.replace('%', "%25").replace('#', "%23"))
}

/// `font-weight` 값. 가변 폰트(wght 축)면 `최소 최대` 범위
fn css_weight(font: &FontRef) -> String {
    let w = |v: f32| v.round().clamp(1.0, 1000.0);
    match font.axes().get_by_tag(Tag::new(b"wght")) {
        Some(a) => format!("{} {}", w(a.min_value()), w(a.max_value())),
        None => w(font.attributes().weight.value()).to_string(),
    }
}

/// `@font-face` 한 덩어리. `files` = (파일 이름, format)
fn font_face(font: &FontRef, fallback_family: &str, files: &[(String, &str)]) -> String {
    let family = [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| font.localized_strings(id).english_or_first())
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_family.to_owned());
    let style = match font.attributes().style {
        Style::Normal => "normal",
        Style::Italic => "italic",
        Style::Oblique(_) => "oblique",
    };
    let src = files
        .iter()
        .map(|(name, format)| format!("url('{}') format('{format}')", css_url(name)))
        .collect::<Vec<_>>()
        .join(",\n       ");
    format!(
        "@font-face {{\n  font-family: '{}';\n  src: {src};\n  font-weight: {};\n  font-style: {style};\n  font-display: swap;\n}}\n",
        css_str(&family),
        css_weight(font)
    )
}

impl Tool for WebFontSubset {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "web.font_subset".into(),
            name: "웹폰트 서브셋".into(),
            category: "web".into(),
            description: "필요한 글자만 남겨 WOFF2 웹폰트로 만듭니다.".into(),
            extensions: ["ttf", "otf", "woff", "woff2"].map(String::from).to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        // 결과가 서로를 가리키는 세트(CSS → 폰트)라 폴더 하나로 둔다: 엔진이 파일 이름을 바꾸지 않는다
        ToolKind::Generator
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(SubsetOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let src = input
            .first()
            .ok_or_else(|| failed("폰트 파일을 넣어 주세요."))?;
        let o: SubsetOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        if !(o.woff2 || o.woff || o.ttf) {
            return Err(ToolError::InvalidOptions(
                "출력 형식을 하나 이상 고르세요.".into(),
            ));
        }
        let chars = chars(&o)?;
        ctx.check_cancel()?;
        let sfnt = to_sfnt(&std::fs::read(src)?)?;
        let font = FontRef::new(&sfnt).map_err(|_| broken())?;
        ctx.progress(0.1);
        let sub = subset(&font, &chars)?;
        ctx.progress(0.6);

        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let dir = format!("{stem}-subset");
        std::fs::create_dir_all(out_dir.join(&dir))?;
        // (파일 이름, CSS format, 내용)
        let mut outs: Vec<(String, &str, Vec<u8>)> = Vec::new();
        if o.woff2 {
            ctx.check_cancel()?;
            let w = ttf2woff2::encode(&sub, ttf2woff2::BrotliQuality::default())
                .map_err(|_| broken())?;
            outs.push((format!("{stem}.woff2"), "woff2", w));
            ctx.progress(0.85);
        }
        if o.woff {
            ctx.check_cancel()?;
            let w = woff::encode(&sub).ok_or_else(broken)?;
            outs.push((format!("{stem}.woff"), "woff", w));
        }
        if o.ttf {
            let (ext, format) = if sub.starts_with(b"OTTO") {
                ("otf", "opentype")
            } else {
                ("ttf", "truetype")
            };
            outs.push((format!("{stem}.{ext}"), format, sub));
        }
        ctx.check_cancel()?;

        let mut names = Vec::new();
        for (name, _, data) in &outs {
            names.push(Path::new(&dir).join(name));
            std::fs::write(out_dir.join(names.last().unwrap()), data)?;
        }
        let files: Vec<(String, &str)> = outs.iter().map(|(n, f, _)| (n.clone(), *f)).collect();
        names.push(Path::new(&dir).join(format!("{stem}.css")));
        std::fs::write(
            out_dir.join(names.last().unwrap()),
            font_face(&font, &stem, &files),
        )?;
        ctx.progress(1.0);
        Ok(ToolOutput::Files(names))
    }
}
