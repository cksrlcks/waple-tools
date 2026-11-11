//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 GIF를 만든다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};
use waple_tools_image_common::testkit::animated_gif;

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-gif-optimize");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_gif_optimize::tool();
    assert_eq!(addon.meta().id, "image.gif_optimize");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.gif");
    animated_gif(&src, 64, 48, 6);
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "colors": 32, "frameStep": 2, "scale": 50 });
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t
            .run(&ctx, std::slice::from_ref(&src), &out, &opts)
            .unwrap()
        else {
            panic!("파일이어야 함");
        };
        std::fs::read(out.join(&names[0])).unwrap()
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
