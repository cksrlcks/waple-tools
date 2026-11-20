use serde_json::json;
use waple_tools_core::ToolKind;

use super::matrix::{MODES, effect};
use super::*;

#[test]
fn effect_transposes_and_keeps_alpha() {
    let t = effect(&MODES[3].m); // 전색맹
    assert_eq!(&t[0..5], &[0.299, 0.299, 0.299, 0.0, 0.0]); // 입력 R 행
    assert_eq!(&t[5..10], &[0.587, 0.587, 0.587, 0.0, 0.0]);
    assert_eq!(t[18], 1.0);
    assert_eq!(t[24], 1.0);
    let p = effect(&MODES[0].m); // 적색맹: 입력 G → 출력 R
    assert_eq!(p[5], 1.052583);
}

#[test]
fn mode_index_defaults_to_deutan() {
    assert_eq!(mode_index(&json!(null)), 1);
    assert_eq!(mode_index(&json!({})), 1);
    assert_eq!(mode_index(&json!({ "mode": "nope" })), 1);
    assert_eq!(mode_index(&json!({ "mode": "protan" })), 0);
    assert_eq!(mode_index(&json!({ "mode": "achroma" })), 3);
}

#[test]
fn hit_zones_at_96_dpi() {
    use super::lens::hit;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let (w, h) = (520, 360);
    assert_eq!(hit(0, 0, w, h, 96), HTTOPLEFT);
    assert_eq!(hit(519, 359, w, h, 96), HTBOTTOMRIGHT);
    assert_eq!(hit(2, 200, w, h, 96), HTLEFT);
    assert_eq!(hit(200, 358, w, h, 96), HTBOTTOM);
    assert_eq!(hit(200, 2, w, h, 96), HTTOP);
    assert_eq!(hit(200, 15, w, h, 96), HTCAPTION); // 손잡이 가운데: 끌어서 이동
    assert_eq!(hit(40, 15, w, h, 96), HTCLIENT); // 보기 방식 이름: 메뉴
    assert_eq!(hit(500, 15, w, h, 96), HTCLIENT); // [닫기]
    assert_eq!(hit(200, 200, w, h, 96), HTCLIENT); // 렌즈 안
}

#[test]
fn hit_zones_scale_with_dpi() {
    use super::lens::hit;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    // 150%: 테두리 6px, 손잡이 45px
    assert_eq!(hit(5, 200, 780, 540, 144), HTLEFT);
    assert_eq!(hit(390, 40, 780, 540, 144), HTCAPTION);
    assert_eq!(hit(390, 50, 780, 540, 144), HTCLIENT);
}

#[test]
fn meta_and_schema() {
    let t = tool();
    assert_eq!(t.meta().id, "a11y.cvd_lens");
    assert_eq!(t.meta().category, "a11y");
    assert_eq!(t.kind(), ToolKind::Interactive);
    let s = t.options_schema();
    assert_eq!(s["x-input"], "none");
    // mode 필드: x-run-buttons, 값 4개 (설명 첫 문단 = 라벨)
    assert_eq!(s["properties"]["mode"]["x-run-buttons"], true);
    let defs = s["$defs"]["LensMode"]["oneOf"].as_array().unwrap();
    assert_eq!(defs.len(), 4);
    assert!(
        defs[0]["description"]
            .as_str()
            .unwrap()
            .starts_with("적색맹\n\n")
    );
}
