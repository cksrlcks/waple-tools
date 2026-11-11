//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use std::path::Path;

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-web-woff2");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_web_woff2::tool();
    assert_eq!(addon.meta().id, "web.woff2");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("mini.ttf");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/mini.ttf");
    std::fs::copy(fixture, &src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t
            .run(&ctx, std::slice::from_ref(&src), &out, &json!({}))
            .unwrap()
        else {
            panic!("파일이어야 함");
        };
        names
            .into_iter()
            .map(|n| (n.clone(), std::fs::read(out.join(n)).unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
