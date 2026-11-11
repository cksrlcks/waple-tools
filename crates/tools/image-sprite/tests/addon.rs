//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use image::{Rgba, RgbaImage};
use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-image-sprite");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_image_sprite::tool();
    assert_eq!(addon.meta().id, "image.sprite");
    assert_eq!(addon.kind(), ToolKind::Combine);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let inputs: Vec<_> = [("a", 5, 3, [255, 0, 0, 255]), ("b", 4, 4, [0, 0, 255, 128])]
        .map(|(n, w, h, c)| {
            let p = dir.path().join(format!("{n}.png"));
            RgbaImage::from_pixel(w, h, Rgba(c)).save(&p).unwrap();
            p
        })
        .to_vec();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "retina": true, "prefix": "icon" });
    let run = |t: &dyn Tool, name: &str| {
        let out = dir.path().join(name);
        std::fs::create_dir(&out).unwrap();
        let ToolOutput::Files(names) = t.run(&ctx, &inputs, &out, &opts).unwrap() else {
            panic!("파일이어야 함");
        };
        names
            .into_iter()
            .map(|n| {
                let bytes = std::fs::read(out.join(&n)).unwrap();
                // PNG는 픽셀로 비교한다: 워크스페이스 빌드와 DLL의 flate2 백엔드가 달라 압축 바이트가 다를 수 있다
                let body = if n.extension().is_some_and(|e| e == "png") {
                    image::load_from_memory(&bytes)
                        .unwrap()
                        .to_rgba8()
                        .into_raw()
                } else {
                    bytes
                };
                (n, body)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
