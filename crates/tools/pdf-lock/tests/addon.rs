//! 추가 설치 방식(DLL)으로 빌드해도 내장 방식과 같은 결과를 만든다.
//! 결과 바이트는 실행마다 키가 달라 다르므로, 같은 비밀번호로 풀린 내용을 비교한다.

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolKind, ToolOutput};
use waple_tools_pdf_common::lopdf::{Document, LoadOptions};
use waple_tools_pdf_common::testkit::{sample_pdf, summary};

#[test]
fn dll_matches_builtin() {
    let dll = build_addon("waple-tools-tool-pdf-lock");
    let addon = unsafe { load(&dll) }.unwrap();
    let builtin = waple_tools_tool_pdf_lock::tool();
    assert_eq!(addon.meta().id, "pdf.lock");
    assert_eq!(addon.kind(), ToolKind::PerFile);
    assert_eq!(addon.options_schema(), builtin.options_schema());

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, sample_pdf(3, "")).unwrap();
    let inputs = vec![src];
    let ctx = JobContext::new(|_| {}, Default::default());
    let opts = json!({ "password": "비번", "passwordConfirm": "비번" });
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
                let mut doc =
                    Document::load_mem_with_options(&bytes, LoadOptions::with_password("비번"))
                        .unwrap();
                let mut plain = vec![];
                doc.save_to(&mut plain).unwrap();
                (n.clone(), summary(&n, &plain))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(&addon, "a"), run(&*builtin, "b"));
}
