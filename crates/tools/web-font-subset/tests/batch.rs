//! 배치 엔진을 거쳐도 결과가 `<이름>-subset/` 폴더 하나로 놓이고, CSS의 url이 그 폴더 안 실제 파일을 가리킨다.
//! (PerFile이면 "원본 옆에 저장" 접미사·`(1)`이 파일 이름에 붙어 CSS가 깨진다)

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use waple_tools_core::batch::{FileResult, JobSpec, OutputPolicy, run};

#[test]
fn css_urls_point_at_placed_files() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("work"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let src = work.join("my font.ttf");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/subset-test.ttf");
    std::fs::copy(fixture, &src).unwrap();
    let original = std::fs::read(&src).unwrap();
    let spec = JobSpec {
        inputs: vec![src.clone()],
        recursive: false,
        policy: OutputPolicy::KeepOriginal {
            suffix: "_min".into(),
        },
        keep_if_larger: true,
        threads: 1,
        opts: serde_json::json!({ "woff": true }),
    };
    let tool = waple_tools_tool_web_font_subset::tool();
    let go = |id: &str| {
        let rep = run(
            &*tool,
            &spec,
            id,
            &logs,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();
        let [FileResult::Done { outputs, .. }] = &rep.files[..] else {
            panic!("{rep:?}");
        };
        outputs.clone()
    };

    for (id, folder) in [("j1", "my font-subset"), ("j2", "my font-subset (1)")] {
        let outputs = go(id);
        let dir = work.join(folder);
        assert_eq!(
            outputs,
            ["my font.woff2", "my font.woff", "my font.css"].map(|n| dir.join(n)),
            "{id}"
        );
        let css = std::fs::read_to_string(dir.join("my font.css")).unwrap();
        for url in ["url('my font.woff2')", "url('my font.woff')"] {
            assert!(css.contains(url), "{css}");
        }
    }
    assert_eq!(std::fs::read(&src).unwrap(), original, "원본 그대로");
}
