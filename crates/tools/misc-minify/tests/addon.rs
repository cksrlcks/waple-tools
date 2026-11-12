//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use std::path::Path;

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-misc-minify");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_misc_minify::tool();
    assert_eq!(addon.meta().id, "misc.minify");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/misc-minify");
    let ctx = JobContext::new(|_| {}, Default::default());
    for name in ["style.css", "app.js", "page.html"] {
        let src = dir.path().join(name);
        std::fs::copy(fixtures.join(name), &src).unwrap();
        let run = |t: &dyn Tool, sub: &str| {
            let out = dir.path().join(format!("{sub}-{name}"));
            std::fs::create_dir(&out).unwrap();
            let ToolOutput::Files(names) = t
                .run(&ctx, std::slice::from_ref(&src), &out, &json!({}))
                .unwrap()
            else {
                panic!("파일이어야 함");
            };
            std::fs::read(out.join(&names[0])).unwrap()
        };
        assert_eq!(run(&addon, "a"), run(&*builtin, "b"), "{name}");
    }
    // 구문 오류도 DLL 경계를 넘어 같은 한국어 메시지
    let src = dir.path().join("broken.js");
    std::fs::copy(fixtures.join("broken.js"), &src).unwrap();
    let out = dir.path().join("err");
    std::fs::create_dir(&out).unwrap();
    let e = addon
        .run(&ctx, std::slice::from_ref(&src), &out, &json!({}))
        .unwrap_err();
    assert!(e.to_string().starts_with("구문 오류(5줄 "), "{e}");
}
