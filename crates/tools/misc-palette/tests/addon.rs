//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-misc-palette");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_misc_palette::tool();
    assert_eq!(addon.meta().id, "misc.palette");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("photo.png");
    waple_tools_image_common::testkit::photo(120, 80)
        .save(&input)
        .unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    for opts in [
        json!({ "count": 5, "sort": "brightness" }),
        json!({ "mode": "roles" }),
    ] {
        let a = addon
            .run(&ctx, std::slice::from_ref(&input), dir.path(), &opts)
            .unwrap();
        let b = builtin
            .run(&ctx, std::slice::from_ref(&input), dir.path(), &opts)
            .unwrap();
        let (ToolOutput::Blocks(a), ToolOutput::Blocks(b)) = (a, b) else {
            panic!("블록이어야 함");
        };
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
}
