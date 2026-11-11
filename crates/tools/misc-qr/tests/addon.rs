//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{Block, JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-misc-qr");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_misc_qr::tool();
    assert_eq!(addon.meta().id, "misc.qr");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    let opts = json!({ "url": "https://example.com", "size": 256 });
    let a = addon.run(&ctx, &[], out.path(), &opts).unwrap();
    let b = builtin.run(&ctx, &[], out.path(), &opts).unwrap();
    let (ToolOutput::Blocks(a), ToolOutput::Blocks(b)) = (a, b) else {
        panic!("블록이어야 함");
    };
    assert_eq!(comparable(a), comparable(b));
}

/// PNG는 픽셀로 비교한다: 워크스페이스 빌드는 flate2 백엔드(zlib-rs)가 DLL(miniz_oxide)과 달라 압축 바이트가 다르다
fn comparable(blocks: Vec<Block>) -> Vec<serde_json::Value> {
    use base64::Engine as _;
    blocks
        .into_iter()
        .map(|b| match b {
            Block::Image { src, name } => {
                let b64 = src.strip_prefix("data:image/png;base64,").unwrap();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .unwrap();
                let mut r = png::Decoder::new(std::io::Cursor::new(bytes))
                    .read_info()
                    .unwrap();
                let mut px = vec![0; r.output_buffer_size().unwrap()];
                r.next_frame(&mut px).unwrap();
                json!({ "image": name, "pixels": px })
            }
            other => serde_json::to_value(other).unwrap(),
        })
        .collect()
}
