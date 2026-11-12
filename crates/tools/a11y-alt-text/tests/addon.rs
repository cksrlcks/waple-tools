//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다. DLL에는 모델이 들어가야 하므로 모델이 있을 때만.
#![cfg(has_model)]

use std::path::Path;

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-a11y-alt-text");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_a11y_alt_text::tool();
    assert_eq!(addon.meta().id, "a11y.alt_text");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/a11y-notice.png");
    let ctx = JobContext::new(|_| {}, Default::default());
    let run = |t: &dyn Tool| {
        let ToolOutput::Blocks(b) = t
            .run(
                &ctx,
                std::slice::from_ref(&src),
                Path::new("."),
                &json!({ "output": "srOnly" }),
            )
            .unwrap()
        else {
            panic!("블록이어야 함");
        };
        serde_json::to_value(b).unwrap()
    };
    assert_eq!(run(&addon), run(&*builtin));
}
