//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use std::path::Path;

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-web-font-subset");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_web_font_subset::tool();
    assert_eq!(addon.meta().id, "web.font_subset");
    assert_eq!(addon.kind(), ToolKind::Generator);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures");
    let ctx = JobContext::new(|_| {}, Default::default());
    // WOFF(zlib)는 빌드마다 flate2 백엔드가 달라 바이트가 다를 수 있어 WOFF2·TTF만 비교한다
    let opts = json!({ "ttf": true, "custom": true, "customText": "똠" });
    for name in ["subset-test.ttf", "subset-test.otf"] {
        let src = dir.path().join(name);
        std::fs::copy(fixtures.join(name), &src).unwrap();
        let run = |t: &dyn Tool, out: &str| {
            let out = dir.path().join(format!("{name}-{out}"));
            std::fs::create_dir(&out).unwrap();
            let ToolOutput::Files(names) = t
                .run(&ctx, std::slice::from_ref(&src), &out, &opts)
                .unwrap()
            else {
                panic!("파일이어야 함");
            };
            names
                .into_iter()
                .map(|n| (n.clone(), std::fs::read(out.join(n)).unwrap()))
                .collect::<Vec<_>>()
        };
        let a = run(&addon, "a");
        assert_eq!(a.len(), 3, "{name}");
        assert_eq!(a, run(&*builtin, "b"), "{name}");
    }
}
