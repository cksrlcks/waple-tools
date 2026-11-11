//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 세트를 만든다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};
use waple_tools_image_common::testkit::photo;

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-icon");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_icon::tool();
    assert_eq!(addon.meta().id, "image.icon");
    assert_eq!(addon.kind(), ToolKind::Generator);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("logo.png");
    photo(256, 256).save(&src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "windows": true, "radius": 20 });
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
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
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
