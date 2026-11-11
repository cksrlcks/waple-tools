//! dev.copy를 추가 설치용 DLL로 빌드해 읽고 실행한다.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use serde_json::json;
use waple_tools_addon_abi::load;
use waple_tools_addon_abi::testkit::build_addon;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolOutput};

fn addon() -> waple_tools_addon_abi::AddonTool {
    let dll = build_addon("waple-tools-tool-dev-copy");
    unsafe { load(&dll) }.expect("DLL 로드")
}

#[test]
fn dll_tool_matches_builtin() {
    let t = addon();
    let b = waple_tools_tool_dev_copy::tool();
    assert_eq!(t.meta().id, b.meta().id);
    assert_eq!(t.meta().name, b.meta().name);
    assert_eq!(t.kind(), ToolKind::PerFile);
    assert_eq!(t.options_schema(), b.options_schema());
}

#[test]
fn dll_tool_runs_and_reports_progress() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("한글 a.txt");
    std::fs::write(&src, b"abc").unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let seen = Arc::new(Mutex::new(vec![]));
    let s = seen.clone();
    let ctx = JobContext::new(move |r| s.lock().unwrap().push(r), Default::default());
    let r = addon().run(&ctx, &[src], &out, &json!({})).unwrap();
    assert!(matches!(r, ToolOutput::Files(f) if f == [PathBuf::from("한글 a.txt")]));
    assert_eq!(std::fs::read(out.join("한글 a.txt")).unwrap(), b"abc");
    assert_eq!(*seen.lock().unwrap(), vec![1.0]);
}

#[test]
fn dll_panic_becomes_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.txt");
    std::fs::write(&src, b"abc").unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = addon().run(&ctx, &[src], dir.path(), &json!({ "panic": true }));
    assert!(matches!(r, Err(ToolError::Failed(m)) if m == "도구 실행 중 오류가 발생했습니다."));
    // 패닉 뒤에도 같은 DLL을 계속 쓸 수 있다
    assert_eq!(addon().meta().id, "dev.copy");
}

#[test]
fn dll_sees_cancel_flag() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.txt");
    std::fs::write(&src, b"abc").unwrap();
    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let r = addon().run(&ctx, &[src], dir.path(), &json!({}));
    assert!(matches!(r, Err(ToolError::Cancelled)));
}
