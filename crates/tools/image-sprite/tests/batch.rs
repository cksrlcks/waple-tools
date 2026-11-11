//! 배치 엔진(Combine)을 거치면 결과가 첫 파일 옆 `sprite/` 폴더 하나로 놓이고, CSS가 그 폴더 안 PNG를 가리킨다.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use image::{Rgba, RgbaImage};
use waple_tools_core::batch::{FileResult, JobSpec, OutputPolicy, run};

#[test]
fn result_folder_sits_next_to_first_input() {
    let d = tempfile::tempdir().unwrap();
    let (work, logs) = (d.path().join("work"), d.path().join("logs"));
    std::fs::create_dir(&work).unwrap();
    let inputs: Vec<_> = ["b", "a"]
        .map(|n| {
            let p = work.join(format!("{n}.png"));
            RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]))
                .save(&p)
                .unwrap();
            p
        })
        .to_vec();
    let spec = JobSpec {
        inputs: inputs.clone(),
        recursive: false,
        policy: OutputPolicy::default(),
        keep_if_larger: true,
        threads: 1,
        opts: serde_json::json!({ "retina": true }),
    };
    let tool = waple_tools_tool_image_sprite::tool();
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

    let first = go("j1");
    let dir = work.join("sprite");
    assert_eq!(
        first,
        ["sprite.png", "sprite@2x.png", "sprite.css"].map(|n| dir.join(n))
    );
    let css = std::fs::read_to_string(dir.join("sprite.css")).unwrap();
    assert!(css.contains("url(sprite.png)") && css.contains("url(sprite@2x.png)"));
    for p in &inputs {
        assert!(p.is_file(), "원본은 그대로");
    }

    // 다시 돌리면 폴더 이름만 바뀌고 안의 파일 이름은 그대로라 CSS의 url이 맞는다
    let again = go("j2");
    let dir2 = work.join("sprite (1)");
    assert_eq!(again[0], dir2.join("sprite.png"));
    assert!(dir2.join("sprite@2x.png").is_file());
    assert!(dir2.join("sprite.css").is_file());
}
