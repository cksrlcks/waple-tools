//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-misc-rename");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_misc_rename::tool();
    assert_eq!(addon.meta().id, "misc.rename");
    assert_eq!(addon.kind(), ToolKind::Rename);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let d = tempfile::tempdir().unwrap();
    let inputs: Vec<_> = ["My Photo.JPG", "b.txt"]
        .iter()
        .map(|n| {
            let p = d.path().join(n);
            std::fs::write(&p, "").unwrap();
            p
        })
        .collect();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({
        "spacesToHyphen": true, "changeCase": true, "caseMode": "lower",
        "number": true, "extCase": "lower"
    });
    let a = addon.run(&ctx, &inputs, d.path(), &opts).unwrap();
    let b = builtin.run(&ctx, &inputs, d.path(), &opts).unwrap();
    let (ToolOutput::Files(a), ToolOutput::Files(b)) = (a, b) else {
        panic!("이름 목록이어야 함");
    };
    assert_eq!(a, b);
    assert_eq!(a[0].to_string_lossy(), "my-photo_001.jpg");
}
