//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-pdf-from-image");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_pdf_from_image::tool();
    assert_eq!(addon.meta().id, "pdf.from_image");
    assert_eq!(addon.kind(), ToolKind::Combine);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let inputs: Vec<_> = ["a.jpg", "b.png"]
        .map(|n| {
            let p = dir.path().join(n);
            waple_tools_image_common::testkit::photo(120, 80)
                .save(&p)
                .unwrap();
            p
        })
        .to_vec();
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({});
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
                (
                    n.clone(),
                    waple_tools_pdf_common::testkit::summary(&n, &bytes),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
