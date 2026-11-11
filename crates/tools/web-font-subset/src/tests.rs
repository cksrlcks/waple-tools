use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use serde_json::json;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider, Tag};
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

/// tests/fixtures/subset-test.{ttf,otf}: ASCII + 한글 몇 자('똠' 포함), 굵게·기울임, GPOS 커닝(A-V).
/// 만드는 법: tests/fixtures/make-subset-fonts.py (fontTools)
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// 결과 폴더 안 상대 경로
fn rel(name: &str) -> PathBuf {
    Path::new("subset-test-subset").join(name)
}

type Out = Vec<(PathBuf, Vec<u8>)>;

/// `name` 파일로 두고 실행한 (결과 이름, 내용) 목록
fn run(name: &str, bytes: &[u8], opts: serde_json::Value) -> Result<Out, ToolError> {
    run_ctx(
        name,
        bytes,
        opts,
        &JobContext::new(|_| {}, Default::default()),
    )
}

fn run_ctx(
    name: &str,
    bytes: &[u8],
    opts: serde_json::Value,
    ctx: &JobContext,
) -> Result<Out, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join(name);
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let result = tool().run(ctx, std::slice::from_ref(&src), &out, &opts);
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    let ToolOutput::Files(names) = result? else {
        panic!("파일이어야 함");
    };
    Ok(names
        .into_iter()
        .map(|n| {
            let data = std::fs::read(out.join(&n)).unwrap();
            (n, data)
        })
        .collect())
}

/// 직접 입력한 글자만, 고른 형식으로
fn only(text: &str, formats: serde_json::Value) -> serde_json::Value {
    let mut o = json!({ "hangul": false, "latin": false, "custom": true, "customText": text, "woff2": false });
    o.as_object_mut()
        .unwrap()
        .extend(formats.as_object().unwrap().clone());
    o
}

fn has(font: &[u8], c: char) -> bool {
    FontRef::new(font).unwrap().charmap().map(c).is_some()
}

fn glyphs(font: &[u8]) -> u16 {
    FontRef::new(font).unwrap().maxp().unwrap().num_glyphs()
}

/// 글자 `c`의 윤곽 (SVG 경로). 합성 글리프는 부품까지 그린다
fn outline(font: &[u8], c: char) -> String {
    use skrifa::instance::{LocationRef, Size};
    use skrifa::outline::{DrawSettings, pen::SvgPen};
    let font = FontRef::new(font).unwrap();
    let gid = font.charmap().map(c).unwrap();
    let mut pen = SvgPen::new();
    font.outline_glyphs()
        .get(gid)
        .unwrap()
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .unwrap();
    pen.to_string()
}

#[test]
fn composite_and_supplementary_plane_chars() {
    for name in ["subset-test.ttf", "subset-test.otf"] {
        let src = fixture(name);
        let out = run(name, &src, only("Á😀", json!({ "ttf": true }))).unwrap();
        let font = &out[0].1;
        assert!(has(font, 'Á') && has(font, '😀'), "{name}");
        assert!(
            !has(font, 'A'),
            "{name}: 부품 글리프는 남아도 cmap에는 없다"
        );
        for c in ['Á', '😀'] {
            let path = outline(font, c);
            assert!(!path.is_empty(), "{name} {c}");
            assert_eq!(path, outline(&src, c), "{name} {c}: 원본과 같은 모양");
        }
    }
    // TTF의 Á는 A + 악센트 합성 글리프: 부품 둘이 함께 남는다 (.notdef·공백·Á·😀·A·악센트)
    let out = run(
        "t.ttf",
        &fixture("subset-test.ttf"),
        only("Á😀", json!({ "ttf": true })),
    )
    .unwrap();
    assert_eq!(glyphs(&out[0].1), 6);
}

#[test]
fn variable_font_weight_is_a_range() {
    let src = fixture("subset-test-var.ttf");
    let out = run(
        "subset-test-var.ttf",
        &src,
        json!({ "ttf": true, "woff2": false }),
    )
    .unwrap();
    let font = FontRef::new(&out[0].1).unwrap();
    assert!(font.table_data(Tag::new(b"fvar")).is_some(), "가변 축 유지");
    let css = String::from_utf8(out[1].1.clone()).unwrap();
    assert!(css.contains("  font-weight: 100 900;\n"), "{css}");
}

#[test]
fn css_escapes() {
    assert_eq!(css_url("a%b#c'd\\e.woff2"), "a%25b%23c\\'d\\\\e.woff2");
    assert_eq!(css_str("A\u{0}B\n'"), "AB\\'");
}

#[test]
fn ksx1001_is_2350_hangul_syllables() {
    let set: BTreeSet<char> = KSX1001.chars().collect();
    assert_eq!(KSX1001.chars().count(), 2350);
    assert_eq!(set.len(), 2350, "중복 없음");
    assert!(set.iter().all(|c| ('가'..='힣').contains(c)));
    assert_eq!(KSX1001.chars().next(), Some('가'));
    assert_eq!(KSX1001.chars().last(), Some('힝'));
    assert!(!set.contains(&'똠'), "완성형에 없는 글자");
}

#[test]
fn keeps_only_chosen_chars_truetype() {
    let src = fixture("subset-test.ttf");
    // '뷁'은 폰트에 없다: 조용히 빠진다
    let out = run(
        "subset-test.ttf",
        &src,
        only("가똠AV뷁", json!({ "ttf": true })),
    )
    .unwrap();
    let names: Vec<_> = out.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(names, [rel("subset-test.ttf"), rel("subset-test.css")]);
    let ttf = &out[0].1;
    for c in ['가', '똠', 'A', 'V', ' '] {
        assert!(has(ttf, c), "{c} 남음");
    }
    for c in ['나', 'B', '뷁'] {
        assert!(!has(ttf, c), "{c} 빠짐");
    }
    assert_eq!(glyphs(ttf), 6, ".notdef + 5글자");
    assert!(ttf.len() < src.len(), "원본보다 작다");
    let font = FontRef::new(ttf).unwrap();
    assert!(
        font.table_data(Tag::new(b"GPOS")).is_some(),
        "커닝(GPOS) 유지"
    );
}

#[test]
fn keeps_only_chosen_chars_cff() {
    let src = fixture("subset-test.otf");
    let out = run(
        "subset-test.otf",
        &src,
        only("가똠A", json!({ "ttf": true })),
    )
    .unwrap();
    assert_eq!(out[0].0, rel("subset-test.otf"));
    let otf = &out[0].1;
    assert!(otf.starts_with(b"OTTO"));
    for c in ['가', '똠', 'A', ' '] {
        assert!(has(otf, c), "{c} 남음");
    }
    assert!(!has(otf, '나'));
    assert_eq!(glyphs(otf), 5);
    assert!(otf.len() < src.len());
    // 윤곽도 남아 있다
    let font = FontRef::new(otf).unwrap();
    let gid = font.charmap().map('가').unwrap();
    assert!(font.outline_glyphs().get(gid).is_some());
}

#[test]
fn all_formats_and_css() {
    let src = fixture("subset-test.ttf");
    let out = run(
        "subset-test.ttf",
        &src,
        json!({ "woff": true, "ttf": true }),
    )
    .unwrap();
    let names: Vec<PathBuf> = out.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(
        names,
        ["woff2", "woff", "ttf", "css"].map(|e| rel(&format!("subset-test.{e}")))
    );
    let ttf = &out[2].1;
    // WOFF2·WOFF를 풀면 같은 글자가 든 폰트
    for packed in [
        wuff::decompress_woff2(&out[0].1).unwrap(),
        wuff::decompress_woff1(&out[1].1).unwrap(),
    ] {
        assert_eq!(glyphs(&packed), glyphs(ttf));
        assert!(has(&packed, '한') && has(&packed, 'z') && has(&packed, ' '));
        assert!(!has(&packed, '똠'), "완성형 밖");
    }
    assert_eq!(wuff::decompress_woff1(&out[1].1).unwrap().len(), ttf.len());

    let css = String::from_utf8(out[3].1.clone()).unwrap();
    assert_eq!(
        css,
        "@font-face {\n  font-family: 'Waple Test';\n  src: url('subset-test.woff2') format('woff2'),\n       url('subset-test.woff') format('woff'),\n       url('subset-test.ttf') format('truetype');\n  font-weight: 700;\n  font-style: italic;\n  font-display: swap;\n}\n"
    );
}

#[test]
fn default_is_woff2_with_hangul_and_latin() {
    let src = fixture("subset-test.otf");
    let out = run("subset-test.otf", &src, json!({})).unwrap();
    assert_eq!(out.len(), 2);
    let font = wuff::decompress_woff2(&out[0].1).unwrap();
    assert!(has(&font, '가') && has(&font, '~'));
    assert!(!has(&font, '똠'));
    let css = String::from_utf8(out[1].1.clone()).unwrap();
    assert!(
        css.contains("src: url('subset-test.woff2') format('woff2');"),
        "{css}"
    );
}

#[test]
fn woff_and_woff2_inputs_are_read_by_content() {
    let ttf = fixture("subset-test.ttf");
    let woff2 = ttf2woff2::encode(&ttf, ttf2woff2::BrotliQuality::default()).unwrap();
    let woff1 = woff::encode(&ttf).unwrap();
    for (name, bytes) in [("a.woff2", &woff2), ("b.ttf", &woff1)] {
        let out = run(name, bytes, only("가", json!({ "ttf": true }))).unwrap();
        assert!(has(&out[0].1, '가') && !has(&out[0].1, '나'), "{name}");
    }
}

#[test]
fn text_file_chars_with_bom() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("page.html");
    let body = "\u{feff}<p>한글</p>\r\n";
    std::fs::write(&txt, body).unwrap();
    let opts = json!({
        "hangul": false, "latin": false, "fromFile": true,
        "textFile": txt.to_string_lossy(), "woff2": false, "ttf": true
    });
    let out = run("subset-test.ttf", &fixture("subset-test.ttf"), opts).unwrap();
    let ttf = &out[0].1;
    for c in ['한', '글', '<', 'p', '/', '>', ' '] {
        assert!(has(ttf, c), "{c}");
    }
    assert!(!has(ttf, '가') && !has(ttf, 'A'));
    assert_eq!(
        std::fs::read(&txt).unwrap(),
        body.as_bytes(),
        "텍스트 파일 그대로"
    );
}

#[test]
fn text_file_errors_are_korean() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("euckr.txt");
    std::fs::write(&txt, [0xB0, 0xA1, 0xB3, 0xAA]).unwrap(); // EUC-KR '가나'
    let src = fixture("subset-test.ttf");
    let e = run(
        "x.ttf",
        &src,
        json!({ "fromFile": true, "textFile": txt.to_string_lossy() }),
    )
    .unwrap_err();
    assert!(e.to_string().contains("UTF-8 텍스트 파일만"), "{e}");
    let e = run("x.ttf", &src, json!({ "fromFile": true, "textFile": "" })).unwrap_err();
    assert!(e.to_string().contains("텍스트 파일을 골라"), "{e}");
    let missing = dir.path().join("none.txt");
    let e = run(
        "x.ttf",
        &src,
        json!({ "fromFile": true, "textFile": missing.to_string_lossy() }),
    )
    .unwrap_err();
    assert!(e.to_string().contains("열 수 없습니다"), "{e}");
}

#[test]
fn empty_ranges_are_errors() {
    let src = fixture("subset-test.ttf");
    let e = run("x.ttf", &src, json!({ "hangul": false, "latin": false })).unwrap_err();
    assert!(e.to_string().contains("남길 글자가 없습니다"), "{e}");
    // 공백·줄바꿈만 적어도 없는 것과 같다
    let e = run("x.ttf", &src, only(" \n", json!({ "ttf": true }))).unwrap_err();
    assert!(e.to_string().contains("남길 글자가 없습니다"), "{e}");
    let e = run("x.ttf", &src, only("뷁", json!({ "ttf": true }))).unwrap_err();
    assert!(e.to_string().contains("하나도 없습니다"), "{e}");
    let e = run("x.ttf", &src, json!({ "woff2": false })).unwrap_err();
    assert!(matches!(e, ToolError::InvalidOptions(_)), "{e}");
}

#[test]
fn bad_files_are_korean_errors() {
    let e = run("x.ttf", b"hello world", json!({})).unwrap_err();
    assert!(
        e.to_string().contains("폰트 파일을 읽을 수 없습니다"),
        "{e}"
    );
    let e = run("x.ttc", b"ttcf\0\0\0\0", json!({})).unwrap_err();
    assert!(e.to_string().contains("TTC"), "{e}");
    let e = run("x.woff2", b"wOF2 broken", json!({})).unwrap_err();
    assert!(e.to_string().contains("손상"), "{e}");
}

#[test]
fn progress_and_cancel() {
    let src = fixture("subset-test.ttf");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s = seen.clone();
    let ctx = JobContext::new(move |p| s.lock().unwrap().push(p), Default::default());
    run_ctx("x.ttf", &src, json!({}), &ctx).unwrap();
    let seen = seen.lock().unwrap();
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
    assert_eq!(seen.last(), Some(&1.0));

    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let e = run_ctx("x.ttf", &src, json!({}), &ctx).unwrap_err();
    assert!(matches!(e, ToolError::Cancelled));
}

#[test]
fn schema_flags() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "none");
    assert_eq!(s["x-report-size"], true);
    assert_eq!(s["x-require-any"], json!(["woff2", "woff", "ttf"]));
    let f = &s["properties"]["textFile"];
    assert_eq!(f["format"], "file");
    assert_eq!(f["x-enabled-when"], json!({ "fromFile": [true] }));
    assert!(
        f["x-file-extensions"]
            .as_array()
            .unwrap()
            .contains(&json!("srt"))
    );
    assert_eq!(s["properties"]["customText"]["x-wide"], true);
}
