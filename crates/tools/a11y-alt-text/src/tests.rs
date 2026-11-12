use std::path::PathBuf;

use serde_json::json;
use waple_tools_core::{Block, JobContext, ToolError, ToolOutput};

use crate::html::{self, Hide};
use crate::layout::{self, Line, Rect};
use crate::regions::{self, Kind, Region};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures")
        .join(name)
}

fn line(text: &str, x0: f32, y0: f32, x1: f32, y1: f32) -> Line {
    Line {
        text: text.into(),
        confidence: 0.99,
        rect: Rect { x0, y0, x1, y1 },
    }
}

fn texts(regions: &[Region]) -> Vec<String> {
    regions.iter().map(|r| r.items.join(" / ")).collect()
}

fn para(t: &str) -> Region {
    Region {
        kind: Kind::Paragraph,
        items: vec![t.into()],
        lines: vec![],
    }
}

// ---------- 후처리 ----------

/// 공지: 큰 제목, 줄바꿈된 한 문장, 이름표로 시작하는 날짜, ※ 주의
fn notice_lines() -> Vec<Line> {
    vec![
        line("시스템 중단 안내", 60.0, 50.0, 420.0, 110.0),
        line("전기설비 점검 작업으로", 60.0, 150.0, 320.0, 182.0),
        line("서비스가 일시 중단될", 60.0, 186.0, 300.0, 218.0),
        line(
            "예정입니다. 이용에 불편을 드려 죄송합니다.",
            60.0,
            222.0,
            560.0,
            254.0,
        ),
        line(
            "점검시간: 2026년 11월 17일 17:30부터",
            60.0,
            300.0,
            500.0,
            332.0,
        ),
        line("2026년 11월 18일 09:00까지", 60.0, 336.0, 380.0, 368.0),
        line(
            "※ 작업 현안에 따라 점검이 늦어질 수 있습니다.",
            60.0,
            430.0,
            480.0,
            456.0,
        ),
    ]
}

#[test]
fn merges_broken_sentences_and_splits_regions() {
    let (_, r) = layout::analyze(notice_lines());
    assert_eq!(
        texts(&r),
        [
            "시스템 중단 안내",
            "전기설비 점검 작업으로 서비스가 일시 중단될 예정입니다. 이용에 불편을 드려 죄송합니다.",
            "점검시간: 2026년 11월 17일 17:30부터 2026년 11월 18일 09:00까지",
            "※ 작업 현안에 따라 점검이 늦어질 수 있습니다.",
        ]
    );
    assert_eq!(r[0].kind, Kind::Title);
    assert!(r[1..].iter().all(|r| r.kind == Kind::Paragraph));
    let labels: Vec<_> = r.iter().map(regions::label).collect();
    assert_eq!(labels, ["제목", "설명", "날짜·시간", "주의사항"]);
}

#[test]
fn input_order_does_not_matter() {
    let mut l = notice_lines();
    l.reverse();
    let (_, a) = layout::analyze(l);
    let (_, b) = layout::analyze(notice_lines());
    assert_eq!(texts(&a), texts(&b));
}

#[test]
fn reads_columns_one_after_another() {
    // 제목 아래 두 단: 같은 높이의 줄이 좌우로 놓여도 왼쪽 단을 다 읽고 오른쪽 단
    let lines = vec![
        line("여름 캠프 안내", 60.0, 40.0, 600.0, 90.0),
        line("일시: 7월 20일", 75.0, 220.0, 340.0, 250.0),
        line("10:00 ~ 16:00", 75.0, 255.0, 240.0, 285.0),
        line("장소: 시민회관", 75.0, 320.0, 330.0, 350.0),
        line("신청 방법", 495.0, 220.0, 600.0, 250.0),
        line("홈페이지에서 신청서를", 495.0, 262.0, 720.0, 292.0),
        line("작성해 주세요.", 495.0, 297.0, 640.0, 327.0),
    ];
    let (_, r) = layout::analyze(lines);
    assert_eq!(
        texts(&r),
        [
            "여름 캠프 안내",
            "일시: 7월 20일 10:00 ~ 16:00",
            "장소: 시민회관",
            "신청 방법 홈페이지에서 신청서를 작성해 주세요.",
        ]
    );
}

#[test]
fn joins_fragments_on_one_row_but_not_across_columns() {
    let rows = layout::join_rows(vec![
        line("문의:", 100.0, 10.0, 160.0, 40.0),
        line("02-123-4567", 175.0, 12.0, 320.0, 40.0),
        // 멀리 떨어진 오른쪽 단
        line("camp@example.com", 600.0, 10.0, 800.0, 40.0),
    ]);
    let t: Vec<_> = rows.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(t, ["문의: 02-123-4567", "camp@example.com"]);
}

#[test]
fn bullet_lines_become_a_list() {
    let lines = vec![
        line("• 신분증 지참", 60.0, 100.0, 300.0, 130.0),
        line("• 주차 공간이 없으니 대중교통을", 60.0, 134.0, 420.0, 164.0),
        line("이용해 주세요", 80.0, 168.0, 260.0, 198.0),
    ];
    let (_, r) = layout::analyze(lines);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, Kind::List);
    assert_eq!(
        r[0].items,
        ["신분증 지참", "주차 공간이 없으니 대중교통을 이용해 주세요"]
    );
}

#[test]
fn far_apart_but_unfinished_sentence_is_joined() {
    // 줄 간격이 문단 간격만큼 넓어도 앞줄이 문장을 끝내지 않았으면 잇는다
    let lines = vec![
        line("행사 당일에는 정문이 아닌", 60.0, 100.0, 400.0, 130.0),
        line("후문으로 입장해 주십시오.", 60.0, 160.0, 380.0, 190.0),
        line("감사합니다.", 60.0, 240.0, 200.0, 270.0),
    ];
    let (_, r) = layout::analyze(lines);
    assert_eq!(
        texts(&r),
        [
            "행사 당일에는 정문이 아닌 후문으로 입장해 주십시오.",
            "감사합니다."
        ]
    );
}

#[test]
fn item_and_sentence_detection() {
    assert!(layout::starts_item("장소: 시민회관"));
    assert!(layout::starts_item("1. 첫째"));
    assert!(layout::starts_item("※ 참고"));
    assert!(!layout::starts_item("17:30부터 시작"));
    assert!(!layout::starts_item("오늘은 날씨가 좋다"));
    assert!(layout::ends_sentence("예정입니다"));
    assert!(layout::ends_sentence("Thank you!"));
    assert!(!layout::ends_sentence("점검 작업으로"));
    assert!(regions::has_phone("문의 02-123-4567"));
    assert!(!regions::has_phone("2026-11-17"));
    assert!(regions::has_date_time("17:30"));
    assert!(!regions::has_date_time("www.example.com"));
}

#[test]
fn fills_only_safe_missing_spaces() {
    assert_eq!(
        layout::tidy_spacing("점검시간:2026년11월 17일17:30부터"),
        "점검시간: 2026년 11월 17일 17:30부터"
    );
    assert_eq!(
        layout::tidy_spacing("예정입니다.이용에"),
        "예정입니다. 이용에"
    );
    assert_eq!(layout::tidy_spacing("※작업"), "※ 작업");
    for keep in [
        "https://a.kr/x",
        "www.example.com",
        "10:00~16:00",
        "v1.2",
        "2026.11.17",
        "camp@example.com",
        "3층",
    ] {
        assert_eq!(layout::tidy_spacing(keep), keep);
    }
}

#[test]
fn keeps_dates_urls_and_symbols() {
    let t = layout::join_text([
        "https://a.kr/x?y=1&z=2  ",
        " mail@a.kr",
        "2026.11.17(화) 17:30~18:00",
    ]);
    assert_eq!(
        t,
        "https://a.kr/x?y=1&z=2 mail@a.kr 2026.11.17(화) 17:30~18:00"
    );
}

// ---------- 대체텍스트 칸 ----------

fn auto_regions() -> Vec<Region> {
    vec![
        Region {
            kind: Kind::Title,
            items: vec!["안내".into()],
            lines: vec![0],
        },
        para("본문입니다."),
        Region {
            kind: Kind::List,
            items: vec!["하나".into(), "둘".into()],
            lines: vec![1],
        },
    ]
}

#[test]
fn alt_box_round_trips_auto_structure() {
    let auto = auto_regions();
    let back = regions::from_alt(&html::alt_text(&auto), &auto);
    assert_eq!(texts(&back), texts(&auto));
    let kinds: Vec<_> = back.iter().map(|r| r.kind).collect();
    assert_eq!(kinds, [Kind::Title, Kind::Paragraph, Kind::List]);
}

#[test]
fn alt_box_edits_split_merge_and_new_text() {
    let auto = auto_regions();
    // 줄바꿈·겹친 공백은 공백 하나, 빈 조각은 버린다, 모르는 글은 문단
    let r = regions::from_alt(
        "안내 |
 본문  입니다. | | 새 글 | 하나 | 둘",
        &auto,
    );
    assert_eq!(texts(&r), ["안내", "본문 입니다.", "새 글", "하나 / 둘"]);
    let kinds: Vec<_> = r.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        [Kind::Title, Kind::Paragraph, Kind::Paragraph, Kind::List]
    );
    // 목록 항목 사이에 다른 글이 끼면 목록이 나뉜다
    let r = regions::from_alt("하나 | 본문입니다. | 둘", &auto);
    assert_eq!(texts(&r), ["하나", "본문입니다.", "둘"]);
}

// ---------- 출력 ----------

#[test]
fn alt_is_one_line_with_separators_between_regions() {
    let r = vec![
        para("시스템 중단 안내"),
        para("줄이\n바뀐   문장."),
        Region {
            kind: Kind::List,
            items: vec!["a".into(), "b".into()],
            lines: vec![],
        },
        para("  "),
    ];
    let alt = html::alt_text(&r);
    assert_eq!(alt, "시스템 중단 안내 | 줄이 바뀐 문장. | a | b");
    assert!(!alt.contains('\n'));
}

#[test]
fn img_tag_escapes_attributes() {
    assert_eq!(
        html::img_tag("a\"b.png", "<\"R&D\"> 'x'"),
        "<img src=\"a&quot;b.png\" alt=\"&lt;&quot;R&amp;D&quot;&gt; &#39;x&#39;\">"
    );
}

fn sample() -> Vec<Region> {
    vec![
        Region {
            kind: Kind::Title,
            items: vec!["시스템 중단 안내".into()],
            lines: vec![],
        },
        para("점검 <긴급> & 공지"),
        Region {
            kind: Kind::List,
            items: vec!["하나".into()],
            lines: vec![],
        },
    ]
}

#[test]
fn sr_only_class_markup() {
    let h = html::sr_only_html(&sample(), "notice.png", 3, &Hide::Class("sr-only"));
    assert_eq!(
        h,
        "<img src=\"notice.png\" alt=\"\">\n\n<div class=\"sr-only\">\n  <h3>시스템 중단 안내</h3>\n  <p>점검 &lt;긴급&gt; &amp; 공지</p>\n  <ul>\n    <li>하나</li>\n  </ul>\n</div>\n"
    );
    let css = html::css("sr-only");
    assert!(css.starts_with(".sr-only {\n  position: absolute;\n"));
    assert!(css.contains("  clip: rect(0, 0, 0, 0);\n"));
    assert!(!css.contains("display") && !css.contains("visibility"));
}

#[test]
fn sr_only_inline_has_same_content() {
    let regions = sample();
    let inline = html::sr_only_html(&regions, "n.png", 2, &Hide::Inline);
    assert!(inline.contains(
        "<div style=\"position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;\">\n"
    ));
    assert!(!inline.contains("class="));
    // 내용은 방식과 무관하게 같다
    assert!(inline.contains(&html::sr_body(&regions, 2)));
    let class = html::sr_only_html(&regions, "n.png", 2, &Hide::Class("x"));
    assert!(class.contains(&html::sr_body(&regions, 2)));
}

#[test]
fn class_names_are_validated() {
    for ok in ["sr-only", "_a", "-x1", "visually_hidden"] {
        assert!(html::valid_class(ok), "{ok}");
    }
    for bad in ["", "1a", "--x", "a b", "a\"b", "한글", "-"] {
        assert!(!html::valid_class(bad), "{bad}");
    }
}

#[test]
fn invalid_class_is_rejected_before_ocr() {
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = crate::tool().run(
        &ctx,
        &[fixture("a11y-notice.png")],
        std::path::Path::new("."),
        &json!({ "output": "srOnly", "styleMode": "class", "className": "1 bad" }),
    );
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
}

#[test]
fn schema_has_defaults_and_conditions() {
    let s = crate::tool().options_schema();
    let p = &s["properties"];
    assert_eq!(p["className"]["default"], "sr-only");
    assert_eq!(p["styleMode"]["default"], "class");
    assert_eq!(p["output"]["default"], "alt");
    assert_eq!(p["heading"]["default"], "h2");
    assert_eq!(p["text"]["x-multiline"], true);
    assert_eq!(p["text"]["x-fill-from"], "ALT");
    assert_eq!(p["text"]["x-copy"], true);
    assert_eq!(p["text"]["x-enabled-when"], json!({ "output": ["alt"] }));
    assert_eq!(s["x-result-panel"], true);
    assert_eq!(
        p["className"]["x-enabled-when"],
        json!({ "output": ["srOnly"], "styleMode": ["class"] })
    );
}

// ---------- 실제 모델 ----------

#[cfg(has_model)]
fn code<'a>(blocks: &'a [Block], lang: &str) -> Option<&'a str> {
    blocks.iter().find_map(|b| match b {
        Block::Code { lang: l, code, .. } if l == lang => Some(code.as_str()),
        _ => None,
    })
}

/// 실제 모델 테스트는 차례로: OCR 캐시는 파일 하나만 기억해서 병렬 테스트가 서로 바꿔 끼운다
#[cfg(has_model)]
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(has_model)]
fn run(name: &str, opts: serde_json::Value) -> Vec<Block> {
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Blocks(b) = crate::tool()
        .run(&ctx, &[fixture(name)], std::path::Path::new("."), &opts)
        .unwrap()
    else {
        panic!("블록이어야 함");
    };
    b
}

#[cfg(has_model)]
#[test]
fn ocr_notice_end_to_end() {
    let _serial = serial();
    let t = std::time::Instant::now();
    let b = run("a11y-notice.png", json!({}));
    eprintln!("처음 실행(모델 로드 포함): {:?}", t.elapsed());
    let alt = code(&b, "ALT").unwrap();
    eprintln!("ALT: {alt}");
    assert_eq!(
        alt,
        "시스템 중단 안내 | 전기설비 점검 작업으로 서비스가 일시 중단될 예정입니다. 이용에 불편을 드려 죄송합니다. | 점검시간: 2026년 11월 17일 17:30부터 2026년 11월 18일 09:00까지 | ※ 작업 현안에 따라 점검이 늦어질 수 있습니다."
    );
    // ALT 방식은 대체텍스트 칸(ALT 블록)만: img 코드는 주지 않는다
    assert!(code(&b, "img").is_none());
    // 카드 [복사]도 ALT 형식, 확인용 그림은 이름 없음(복사·저장 버튼 없음)
    assert!(
        b.iter()
            .any(|x| matches!(x, Block::Records { copy: Some(c), .. } if c == alt))
    );
    assert!(
        b.iter()
            .any(|x| matches!(x, Block::Image { name, .. } if name.is_empty()))
    );

    // 출력 방식만 바꾸면 OCR을 다시 하지 않는다 (같은 분석 결과를 그대로 쓴다)
    let t = std::time::Instant::now();
    let sr = run(
        "a11y-notice.png",
        json!({ "output": "srOnly", "styleMode": "class", "className": "blind" }),
    );
    assert!(
        t.elapsed() < std::time::Duration::from_millis(300),
        "{:?}",
        t.elapsed()
    );
    let h = code(&sr, "HTML").unwrap();
    eprintln!("{h}");
    assert!(h.starts_with("<img src=\"a11y-notice.png\" alt=\"\">\n\n<div class=\"blind\">\n  <h2>시스템 중단 안내</h2>\n  <p>전기설비"));
    assert!(code(&sr, "CSS").unwrap().starts_with(".blind {"));
    assert!(
        code(&sr, "HTML + CSS")
            .unwrap()
            .starts_with("<style>\n.blind {")
    );

    let inline = run(
        "a11y-notice.png",
        json!({ "output": "srOnly", "styleMode": "inline" }),
    );
    assert!(
        code(&inline, "HTML")
            .unwrap()
            .contains("<div style=\"position:absolute;")
    );
    assert!(code(&inline, "CSS").is_none());
}

#[cfg(has_model)]
#[test]
fn ocr_event_mixed_language_and_columns() {
    let _serial = serial();
    let b = run("a11y-event.png", json!({}));
    let alt = code(&b, "ALT").unwrap();
    eprintln!("ALT: {alt}");
    for must in [
        "Summer Coding Camp 2026",
        "여름방학 코딩 캠프 참가자 모집",
        "2026년 7월 20일",
        "10:00~16:00",
        "www.example.com/camp",
        "02-123-4567",
        "camp@example.com",
    ] {
        assert!(alt.contains(must), "{must} 없음: {alt}");
    }
    // 왼쪽 상자를 다 읽은 뒤 오른쪽 상자
    let at = |s: &str| alt.find(s).unwrap();
    assert!(at("대상") < at("신청"), "{alt}");
    // 다른 상자의 정보는 ` | `로 나뉜다
    assert!(alt.contains("5~6학년 | 신청"), "{alt}");
    assert!(at("Summer") < at("일시"), "{alt}");
}

#[cfg(has_model)]
#[test]
fn edited_alt_box_drives_every_output() {
    let _serial = serial();
    let b = run(
        "a11y-notice.png",
        json!({ "text": "시스템 중단 안내 | 본문 하나
이어짐 | 본문 둘" }),
    );
    assert_eq!(
        code(&b, "ALT").unwrap(),
        "시스템 중단 안내 | 본문 하나 이어짐 | 본문 둘"
    );
    let sr = run(
        "a11y-notice.png",
        json!({ "text": "시스템 중단 안내 | 본문 하나 | 본문 둘", "output": "srOnly" }),
    );
    // 자동 분석의 제목과 같은 글은 제목으로
    assert!(code(&sr, "HTML").unwrap().contains(
        "  <h2>시스템 중단 안내</h2>
  <p>본문 하나</p>
  <p>본문 둘</p>
"
    ));
}

#[cfg(has_model)]
#[test]
fn reads_jpg_and_webp_too() {
    let _serial = serial();
    let png = run("a11y-notice.png", json!({}));
    let want = code(&png, "ALT").unwrap().to_string();
    let dir = tempfile::tempdir().unwrap();
    let img = image::open(fixture("a11y-notice.png")).unwrap();
    for name in ["n.jpg", "n.webp"] {
        let p = dir.path().join(name);
        img.save(&p).unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        let ToolOutput::Blocks(b) = crate::tool()
            .run(&ctx, &[p], dir.path(), &json!({}))
            .unwrap()
        else {
            panic!("블록이어야 함");
        };
        let alt = code(&b, "ALT").unwrap();
        if name.ends_with(".webp") {
            // 무손실이면 같은 글
            assert_eq!(alt, want);
        } else {
            // JPEG 압축 손실로 작은 회색 글씨는 조금 달라질 수 있다: 큰 글씨 영역만 비교
            let head = |s: &str| s.split(" | ").take(3).collect::<Vec<_>>().join(" | ");
            assert_eq!(head(alt), head(&want), "{name}");
        }
    }
    // 원본은 그대로, 결과 파일은 만들지 않는다
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn det_input_size_keeps_tall_posters_readable() {
    use crate::ocr::det_size;
    // 작은 배너는 그대로(32 배수)
    assert_eq!(det_size(720, 520), (736, 512));
    // 세로로 긴 공지: 긴 변이 아니라 넓이로 줄여 폭이 충분히 남는다
    let (w, h) = det_size(1080, 5000);
    assert!(w >= 700 && h <= 4000, "{w}×{h}");
    assert!(w as f32 * h as f32 <= 2_600_000.0);
    // 아주 작은 이미지는 짧은 변 64까지 키운다
    assert_eq!(det_size(400, 20).1, 64);
}

#[cfg(has_model)]
#[test]
fn auto_alt_in_the_box_gives_the_same_html() {
    let _serial = serial();
    let auto = run("a11y-notice.png", json!({ "output": "srOnly" }));
    let text = code(&auto, "ALT").unwrap().to_string();
    let filled = run(
        "a11y-notice.png",
        json!({ "output": "srOnly", "text": text }),
    );
    assert_eq!(code(&filled, "HTML"), code(&auto, "HTML"));
    assert_eq!(code(&filled, "ALT"), code(&auto, "ALT"));
}
