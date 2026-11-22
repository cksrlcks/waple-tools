use waple_tools_core::ToolKind;

use super::analyze::{Rect, Report, analyze, ratio};
use super::*;

const W: usize = 240;
const H: usize = 160;

type Rgb = (u8, u8, u8);

fn canvas(bg: Rgb) -> Vec<u8> {
    [bg.2, bg.1, bg.0, 255].repeat(W * H)
}

fn put(img: &mut [u8], x: usize, y: usize, c: Rgb) {
    img[(y * W + x) * 4..(y * W + x) * 4 + 3].copy_from_slice(&[c.2, c.1, c.0]);
}

fn rect(img: &mut [u8], x: usize, y: usize, w: usize, h: usize, c: Rgb) {
    for yy in y..y + h {
        for xx in x..x + w {
            put(img, xx, yy, c);
        }
    }
}

/// "ㅁ" 모양 글자 하나 (12×14, 획 2px). `aa`가 있으면 바깥에 안티에일리어싱 같은 중간색 한 줄
fn glyph(img: &mut [u8], x: usize, y: usize, c: Rgb, aa: Option<Rgb>) {
    if let Some(a) = aa {
        let bg = img_px(img, x - 2, y - 2);
        rect(img, x - 1, y - 1, 14, 16, a);
        rect(img, x + 2, y + 2, 8, 10, bg);
    }
    rect(img, x, y, 12, 2, c);
    rect(img, x, y + 12, 12, 2, c);
    rect(img, x, y, 2, 14, c);
    rect(img, x + 10, y, 2, 14, c);
}

fn img_px(img: &[u8], x: usize, y: usize) -> Rgb {
    let i = (y * W + x) * 4;
    (img[i + 2], img[i + 1], img[i])
}

/// 한 줄 글자 몇 개
fn word(img: &mut [u8], x: usize, y: usize, n: usize, c: Rgb) {
    for k in 0..n {
        glyph(img, x + k * 15, y, c, None);
    }
}

/// 흰 바탕 위 노이즈 정사각형 (칸 경계에 맞춤: 48..144)
fn noise(img: &mut [u8]) {
    let mut s = 12345u32;
    for y in 48..144 {
        for x in 48..144 {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let v = (s >> 8).to_le_bytes();
            put(img, x, y, (v[0], v[1], v[2]));
        }
    }
}

const WHITE: Rgb = (255, 255, 255);
/// 흰 바탕에 2.85:1
const GRAY99: Rgb = (0x99, 0x99, 0x99);
const BOTH: Crit = Crit::START;
const TEXT_ONLY: Crit = Crit {
    large: false,
    ..Crit::START
};
const LARGE_ONLY: Crit = Crit {
    text: false,
    ..Crit::START
};

/// OCR이 `word(x, y, n)`에 줄 법한 글자 줄 상자 (글자보다 조금 넓다)
fn word_box(x: usize, y: usize, n: usize) -> Rect {
    Rect {
        x: x as i32 - 3,
        y: y as i32 - 3,
        w: (n * 15 + 3) as i32,
        h: 20,
    }
}

fn run(img: &[u8], crit: Crit, text: &[Rect]) -> Report {
    analyze(img, W, H, 24, crit, text)
}

#[test]
fn ratio_black_white_is_21() {
    let r = ratio(1.0, 0.0);
    assert!((r - 21.0).abs() < 1e-4);
    assert_eq!(ratio(0.0, 1.0), r);
}

#[test]
fn black_text_with_antialias_passes() {
    let mut img = canvas(WHITE);
    for k in 0..4 {
        glyph(&mut img, 30 + k * 15, 40, (0, 0, 0), Some((160, 160, 160)));
    }
    let r = run(&img, BOTH, &[word_box(30, 40, 4)]);
    assert!(r.fails.is_empty(), "{:?}", r.fails);
    assert_eq!(r.passes.len(), 1);
    assert!(r.passes[0].ratio > 20.0);
}

#[test]
fn light_gray_text_fails_with_ratio() {
    let mut img = canvas(WHITE);
    word(&mut img, 30, 40, 4, GRAY99);
    let b = word_box(30, 40, 4);
    let r = run(&img, BOTH, &[b]);
    assert_eq!(r.fails.len(), 1);
    assert!(
        (r.fails[0].ratio - 2.85).abs() < 0.02,
        "{}",
        r.fails[0].ratio
    );
    assert_eq!(r.fails[0].rect, b);
    assert!(r.passes.is_empty());
}

#[test]
fn text_just_below_4_5_depends_on_criterion() {
    let mut img = canvas(WHITE);
    word(&mut img, 30, 40, 3, (0x77, 0x77, 0x77)); // 4.48:1
    let b = [word_box(30, 40, 3)];
    assert_eq!(run(&img, BOTH, &b).fails.len(), 1);
    let r = run(&img, LARGE_ONLY, &b);
    assert!(r.fails.is_empty());
    assert_eq!(r.passes.len(), 1);
    // #767676 = 4.54:1 → 4.5 통과
    let mut img = canvas(WHITE);
    word(&mut img, 30, 40, 3, (0x76, 0x76, 0x76));
    assert!(run(&img, BOTH, &b).fails.is_empty());
}

#[test]
fn thin_gray_stroke_in_text_box_passes() {
    // 화면 글꼴의 가는 세로획(ㅣ)은 회색으로만 그려진다: 같은 줄의 진한 획으로 판정
    let mut img = canvas(WHITE);
    glyph(&mut img, 30, 40, (0, 0, 0), None);
    rect(&mut img, 44, 40, 1, 14, (0x88, 0x88, 0x88)); // 3.5:1
    let r = run(&img, BOTH, &[word_box(30, 40, 2)]);
    assert!(r.fails.is_empty(), "{:?}", r.fails);
    assert_eq!(r.passes.len(), 1);
}

#[test]
fn same_luminance_different_hue_fails() {
    // 빨강(휘도 0.2126)과 비슷한 휘도의 초록 배경
    let mut img = canvas((0, 148, 0));
    word(&mut img, 30, 40, 3, (255, 0, 0));
    let r = run(&img, BOTH, &[word_box(30, 40, 3)]);
    assert_eq!(r.fails.len(), 1);
    assert!(r.fails[0].ratio < 1.2, "{}", r.fails[0].ratio);
}

#[test]
fn text_on_card_uses_card_background() {
    let mut img = canvas(WHITE);
    rect(&mut img, 24, 24, 192, 96, (0x1e, 0x40, 0xaf));
    word(&mut img, 50, 50, 3, WHITE); // 흰 글자 / 남색 ≈ 8.7:1
    word(&mut img, 50, 90, 3, (0x4a, 0x6a, 0xd0)); // 같은 계열 파랑 글자 ≈ 1.8:1
    let r = run(&img, BOTH, &[word_box(50, 50, 3), word_box(50, 90, 3)]);
    assert_eq!(r.fails.len(), 1, "{:?}", r.fails);
    assert_eq!(r.fails[0].rect.y, 87);
    assert_eq!(r.passes.len(), 1);
}

#[test]
fn text_on_busy_background_is_unclear() {
    let mut img = canvas(WHITE);
    noise(&mut img);
    let on_noise = Rect {
        x: 60,
        y: 60,
        w: 60,
        h: 20,
    };
    let r = run(&img, BOTH, &[on_noise]);
    assert!(r.fails.is_empty() && r.passes.is_empty(), "{r:?}");
    assert_eq!(r.unclear, [on_noise]);
}

#[test]
fn big_text_uses_background_around_it() {
    // 큰 굵은 글자: 상자 안은 글자색이 더 많아도, 가장자리 띠의 배경으로 판정한다
    let mut img = canvas(WHITE);
    rect(&mut img, 40, 40, 120, 60, (0x77, 0x77, 0x77)); // 상자를 거의 채우는 큰 글자 (4.48:1)
    let b = Rect {
        x: 34,
        y: 34,
        w: 132,
        h: 72,
    };
    let r = run(&img, BOTH, &[b]);
    assert_eq!(r.fails.len(), 1, "{r:?}");
    assert!((r.fails[0].ratio - 4.48).abs() < 0.02);
}

#[test]
fn text_box_outside_image_is_clipped() {
    let mut img = canvas(WHITE);
    word(&mut img, 2, 2, 2, GRAY99);
    let r = run(&img, BOTH, &[word_box(2, 2, 2)]);
    assert_eq!(r.fails.len(), 1);
    assert_eq!((r.fails[0].rect.x, r.fails[0].rect.y), (0, 0));
}

#[test]
fn icons_outside_text_use_3_to_1() {
    let mut img = canvas(WHITE);
    glyph(&mut img, 30, 40, (0x88, 0x88, 0x88), None); // 3.5:1 아이콘
    glyph(&mut img, 150, 40, GRAY99, None); // 2.85:1 아이콘
    // 4.5:1을 켜도 아이콘은 3:1
    let r = run(&img, BOTH, &[]);
    assert_eq!(r.passes.len(), 1, "{r:?}");
    assert_eq!(r.fails.len(), 1);
    assert_eq!(r.fails[0].rect.x, 150);
}

#[test]
fn outline_icon_with_antialias_is_icon() {
    let mut img = canvas(WHITE);
    glyph(&mut img, 60, 60, (0, 0, 0), Some((160, 160, 160)));
    let r = run(&img, BOTH, &[]);
    assert_eq!(r.passes.len(), 1, "{r:?}");
}

#[test]
fn two_color_icon_is_icon() {
    // 회색 동전 위 빨간 ×
    let mut img = canvas(WHITE);
    rect(&mut img, 60, 60, 20, 20, (0x77, 0x77, 0x77));
    for k in 0..20 {
        rect(&mut img, 60 + k, 60 + k, 2, 1, (0xdd, 0, 0));
        rect(&mut img, 78 - k, 60 + k, 2, 1, (0xdd, 0, 0));
    }
    let r = run(&img, BOTH, &[]);
    assert_eq!(r.fails.len() + r.passes.len(), 1, "{r:?}");
}

#[test]
fn pictures_are_not_icons() {
    let mut img = canvas(WHITE);
    // 여러 색 그림 조각 (40×40, 2줄마다 다른 색)
    let palette = [
        (200, 30, 30),
        (30, 160, 60),
        (40, 60, 200),
        (220, 180, 20),
        (140, 40, 160),
        (20, 170, 180),
        (90, 90, 90),
        (230, 110, 30),
    ];
    for row in 0..20 {
        rect(
            &mut img,
            30,
            30 + row * 2,
            40,
            2,
            palette[row % palette.len()],
        );
    }
    // 단색이지만 큰 그림 (100×50 테두리)
    rect(&mut img, 120, 60, 100, 2, (0x33, 0x33, 0x33));
    rect(&mut img, 120, 108, 100, 2, (0x33, 0x33, 0x33));
    rect(&mut img, 120, 60, 2, 50, (0x33, 0x33, 0x33));
    rect(&mut img, 218, 60, 2, 50, (0x33, 0x33, 0x33));
    let r = run(&img, BOTH, &[]);
    assert!(r.fails.is_empty() && r.passes.is_empty(), "{r:?}");
}

#[test]
fn far_icons_are_separate_boxes() {
    let mut img = canvas(WHITE);
    word(&mut img, 10, 20, 2, GRAY99);
    word(&mut img, 150, 120, 2, GRAY99);
    assert_eq!(run(&img, BOTH, &[]).fails.len(), 2);
}

#[test]
fn large_surfaces_are_not_icons() {
    let mut img = canvas(WHITE);
    // 칸 경계에 걸친 낮은 대비 카드 배경 · 진한 머리 띠 · 1px 구분선
    rect(&mut img, 37, 29, 170, 90, GRAY99);
    rect(&mut img, 0, 130, W, 20, (0x1e, 0x40, 0xaf));
    rect(&mut img, 10, 155, 220, 1, GRAY99);
    // 짧은 조각이 이어진 세로선 (스크롤바 가장자리)
    for k in 0..4 {
        rect(&mut img, 232, 10 + k * 22, 1, 20, (0x88, 0xaa, 0xdd));
    }
    let r = run(&img, BOTH, &[]);
    assert!(r.fails.is_empty(), "{:?}", r.fails);
}

#[test]
fn noisy_tiles_have_no_icons() {
    let mut img = canvas(WHITE);
    noise(&mut img);
    let r = run(&img, BOTH, &[]);
    assert!(r.fails.is_empty() && r.passes.is_empty(), "{r:?}");
}

#[test]
fn lens_toggles_keep_one_criterion() {
    assert_eq!(BOTH.toggle_text(), LARGE_ONLY);
    assert_eq!(BOTH.toggle_large(), TEXT_ONLY);
    assert_eq!(LARGE_ONLY.toggle_text(), BOTH);
    // 마지막 하나는 끄지 않는다
    assert_eq!(LARGE_ONLY.toggle_large(), LARGE_ONLY);
    assert_eq!(TEXT_ONLY.toggle_text(), TEXT_ONLY);
}

#[test]
fn labels() {
    assert_eq!(fail_label(4.48), "4.4:1 미달");
    assert_eq!(fail_label(2.85), "2.8:1 미달");
    assert_eq!(fmt_ratio(11.79), "11.7:1");
}

#[test]
fn bar_zones() {
    use super::lens::{BarButton, bar_button, hit};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let (w, h) = (560, 380);
    assert_eq!(hit(0, 0, w, h, 96), HTTOPLEFT);
    assert_eq!(hit(559, 379, w, h, 96), HTBOTTOMRIGHT);
    assert_eq!(hit(2, 200, w, h, 96), HTLEFT);
    assert_eq!(hit(40, 15, w, h, 96), HTCLIENT); // [4.5:1]
    assert_eq!(hit(250, 15, w, h, 96), HTCAPTION); // 가운데: 끌어 옮긴다
    assert_eq!(bar_button(20, 15, w, 96), Some(BarButton::Text));
    assert_eq!(bar_button(100, 15, w, 96), Some(BarButton::Large));
    assert_eq!(bar_button(170, 15, w, 96), None);
    assert_eq!(hit(540, 15, w, h, 96), HTCLIENT); // [닫기]
    assert_eq!(hit(480, 15, w, h, 96), HTCLIENT); // [검사]
    assert_eq!(bar_button(540, 15, w, 96), Some(BarButton::Close));
    assert_eq!(bar_button(480, 15, w, 96), Some(BarButton::Check));
    assert_eq!(bar_button(400, 15, w, 96), None);
    assert_eq!(bar_button(540, 100, w, 96), None);
    assert_eq!(hit(390, 45, 780, 540, 144), HTCLIENT); // 150%: 손잡이 45px 아래
    assert_eq!(hit(390, 40, 780, 540, 144), HTCAPTION);
}

#[test]
fn min_size_keeps_buttons_reachable() {
    use super::lens::{BarButton, bar_button, hit, min_size};
    use windows_sys::Win32::UI::WindowsAndMessaging::HTCAPTION;
    let (w, _) = min_size(96);
    assert_eq!(bar_button(w - 20, 15, w, 96), Some(BarButton::Close));
    assert_eq!(bar_button(w - 80, 15, w, 96), Some(BarButton::Check));
    assert_eq!(bar_button(20, 15, w, 96), Some(BarButton::Text));
    assert_eq!(hit(180, 15, w, 400, 96), HTCAPTION); // 토글과 [검사] 사이에 끌 자리가 남는다
    assert_eq!(min_size(144).0, w * 3 / 2);
}

#[test]
fn meta_and_schema() {
    let t = tool();
    assert_eq!(t.meta().id, "a11y.contrast_lens");
    assert_eq!(t.meta().category, "a11y");
    assert_eq!(t.kind(), ToolKind::Interactive);
    let s = t.options_schema();
    assert_eq!(s["x-input"], "none");
    assert_eq!(s["properties"]["action"]["x-run-buttons"], true);
    // 기준은 렌즈 손잡이에서 고른다: 옵션은 실행 버튼 하나
    let keys: Vec<&String> = s["properties"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["action"]);
    assert_eq!(Crit::START.need(), 4.5);
}

/// 실제 모델로 글자 상자를 찾아 판정까지 (모델 파일이 있을 때만)
#[cfg(has_model)]
#[test]
fn real_model_text_boxes_feed_analysis() {
    let mut img = canvas(WHITE);
    for row in 0..3 {
        word(&mut img, 20, 30 + row * 30, 12, (0x22, 0x22, 0x22));
    }
    let boxes = ocr::text_boxes(&img, W, H).unwrap();
    assert!(boxes.iter().all(|b| b.w > 0 && b.h > 0));
    let r = run(&img, BOTH, &boxes);
    // 모델이 무엇을 글자로 보든 진한 무늬는 미달이 아니고, 어딘가에서 잡힌다
    assert!(r.fails.is_empty(), "{:?}", r.fails);
    assert!(!r.passes.is_empty());
}
