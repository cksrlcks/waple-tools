//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다. DLL에는 모델이 들어가야 하므로 모델이 있을 때만.
#![cfg(has_model)]

use std::path::Path;

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-upscale");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_upscale::tool();
    assert_eq!(addon.meta().id, "image.upscale");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/red-ball.jpg");
    let dir = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t
            .run(
                &ctx,
                std::slice::from_ref(&src),
                &out,
                &json!({ "scale": "x2" }),
            )
            .unwrap()
        else {
            panic!("파일이어야 함");
        };
        let img = image::open(out.join(&names[0])).unwrap().to_rgb8();
        (names, img.dimensions(), img.into_raw())
    };
    let (a, b) = (run(&addon, "a"), run(&*builtin, "b"));
    assert_eq!(a.1, (400, 300));
    assert_eq!((&a.0, a.1), (&b.0, b.1));
    // onnxruntime 멀티스레드 연산은 실행마다 마지막 비트가 다를 수 있다
    let worst = a.2.iter().zip(&b.2).map(|(x, y)| x.abs_diff(*y)).max();
    assert!(worst <= Some(3), "최대 차이 {worst:?}");
}
