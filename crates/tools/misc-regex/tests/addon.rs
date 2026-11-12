//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-misc-regex");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_misc_regex::tool();
    assert_eq!(addon.meta().id, "misc.regex");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    let opts = json!({
        "pattern": r"(?<y>\d{4})-(\d{2})",
        "text": "기간 2024-05\n다음 2025-11"
    });
    let a = addon.run(&ctx, &[], out.path(), &opts).unwrap();
    let b = builtin.run(&ctx, &[], out.path(), &opts).unwrap();
    let (ToolOutput::Blocks(a), ToolOutput::Blocks(b)) = (a, b) else {
        panic!("블록이어야 함");
    };
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}
