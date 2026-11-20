//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 도구 정보를 낸다.
//! `run`은 렌즈 창을 띄우므로 부르지 않는다.

use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{Tool, ToolKind};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-a11y-cvd-lens");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_a11y_cvd_lens::tool();
    assert_eq!(addon.meta().id, "a11y.cvd_lens");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());
}
