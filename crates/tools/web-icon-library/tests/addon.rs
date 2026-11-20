//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 낸다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{Block, JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-web-icon-library");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_web_icon_library::tool();
    assert_eq!(addon.meta().id, "web.icon_library");
    assert_eq!(addon.kind(), ToolKind::Interactive);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    // 픽스처 세트를 test/1.0.0으로 받아 둔 것처럼
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().join("test/1.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/icons/package"
    );
    for n in ["icons.json", "info.json", "metadata.json"] {
        std::fs::copy(std::path::Path::new(fixture).join(n), dir.join(n)).unwrap();
    }
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tempfile::tempdir().unwrap();
    let opts = json!({
        "set": "test", "selected": "home",
        "__resources": { "dir": data.path(), "installed": { "test": "1.0.0" } }
    });
    let a = addon.run(&ctx, &[], out.path(), &opts).unwrap();
    let b = builtin.run(&ctx, &[], out.path(), &opts).unwrap();
    let (ToolOutput::Blocks(a), ToolOutput::Blocks(b)) = (a, b) else {
        panic!("블록이어야 함");
    };
    assert_eq!(a.len(), 3);
    assert_eq!(comparable(a), comparable(b));
}

/// PNG 압축 바이트는 빌드마다 압축 백엔드가 달라 다를 수 있어 PNG 복사 버튼은 데이터 URL 머리만 비교한다
fn comparable(blocks: Vec<Block>) -> Vec<serde_json::Value> {
    blocks
        .into_iter()
        .map(|b| {
            let mut v = serde_json::to_value(b).unwrap();
            if let Some(items) = v["items"].as_array_mut() {
                for it in items.iter_mut().filter(|i| i["kind"] == "copyImage") {
                    let src = it["src"].as_str().unwrap();
                    assert!(src.starts_with("data:image/png;base64,"));
                    it["src"] = json!("png");
                }
            }
            v
        })
        .collect()
}
