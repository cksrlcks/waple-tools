//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use std::path::Path;

use resvg::tiny_skia::Pixmap;
use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-svg");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_svg::tool();
    assert_eq!(addon.meta().id, "image.svg");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("square.svg");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/square.svg");
    std::fs::copy(fixture, &src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let mut n = 0;
    let mut run = |t: &dyn Tool, opts: &serde_json::Value| {
        n += 1;
        let out = dir.path().join(n.to_string());
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t.run(&ctx, std::slice::from_ref(&src), &out, opts).unwrap()
        else {
            panic!("파일이어야 함");
        };
        let bytes = std::fs::read(out.join(&names[0])).unwrap();
        // PNG는 픽셀로 비교한다: 워크스페이스 빌드와 DLL의 압축 백엔드가 달라 바이트가 다를 수 있다
        let bytes = match Pixmap::decode_png(&bytes) {
            Ok(p) => p.data().to_vec(),
            Err(_) => bytes,
        };
        (names, bytes)
    };
    for opts in [json!({}), json!({ "mode": "png", "scale": 3 })] {
        assert_eq!(run(&addon, &opts), run(&*builtin, &opts));
    }
}
