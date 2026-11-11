//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use image::{Rgba, RgbaImage};
use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};
use waple_tools_image_common::testkit::photo;

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-watermark");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_watermark::tool();
    assert_eq!(addon.meta().id, "image.watermark");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("photo.png");
    photo(320, 240).save(&src).unwrap();
    let logo = dir.path().join("logo.png");
    RgbaImage::from_fn(16, 16, |x, y| Rgba([255, 0, 0, ((x + y) * 8) as u8]))
        .save(&logo)
        .unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "kind": "logo", "logo": logo, "position": "tile" });
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t
            .run(&ctx, std::slice::from_ref(&src), &out, &opts)
            .unwrap()
        else {
            panic!("파일이어야 함");
        };
        // PNG는 픽셀로 비교한다: 워크스페이스 빌드와 DLL의 압축 백엔드가 달라 바이트가 다를 수 있다
        let img = image::open(out.join(&names[0])).unwrap().to_rgba8();
        (names, img.into_raw())
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
