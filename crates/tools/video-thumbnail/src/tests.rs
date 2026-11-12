use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use waple_tools_core::{Block, JobContext, ToolError, ToolOutput};
use waple_tools_video_common::testkit::{make_marked, make_ramp, make_video, ramp_level};

use super::*;

/// 4초, 320×240, 위 절반이 시간 따라 밝아지는 영상
fn ramp() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("ramp.mp4");
    make_ramp(&src, 320, 240, 4);
    (dir, src)
}

/// 뽑아서 (결과 이름들, 임시 폴더). 원본이 그대로인지 확인한다
fn run(src: &Path, opts: Value) -> Result<(Vec<String>, tempfile::TempDir), ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let before = std::fs::read(src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &[src.to_path_buf()], dir.path(), &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(std::fs::read(src).unwrap(), before, "원본 그대로");
    let names = names
        .iter()
        .map(|n| n.to_string_lossy().replace('\\', "/"))
        .collect();
    Ok((names, dir))
}

/// 결과 이미지의 크기와 위쪽 밝기
fn look(dir: &Path, name: &str) -> ((u32, u32), f64) {
    let img = image::open(dir.join(name)).unwrap().to_rgb8();
    (img.dimensions(), f64::from(img.get_pixel(8, 8)[0]))
}

/// 한 장 뽑기: (결과 이름, 그 이미지의 크기·위쪽 밝기)
fn shot(src: &Path, opts: Value) -> (String, (u32, u32), f64) {
    let (names, out) = run(src, opts).unwrap();
    assert_eq!(names.len(), 1, "{names:?}");
    let (size, level) = look(out.path(), &names[0]);
    (names[0].clone(), size, level)
}

#[test]
fn probe_returns_duration_and_fps() {
    let (_d, src) = ramp();
    let dir = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Blocks(blocks) = tool()
        .run(&ctx, &[src], dir.path(), &json!({ "x-probe": true }))
        .unwrap()
    else {
        panic!("블록이어야 함");
    };
    let [
        Block::Number {
            label: l1,
            value: secs,
            unit: u1,
        },
        Block::Number {
            label: l2,
            value: fps,
            unit: u2,
        },
    ] = &blocks[..]
    else {
        panic!("숫자 블록 둘");
    };
    assert_eq!((l1.as_str(), u1.as_deref()), ("duration", Some("s")));
    assert!((secs - 4.0).abs() < 0.1, "{secs}");
    assert_eq!((l2.as_str(), u2.as_deref()), ("fps", None));
    assert!((fps - 15.0).abs() < 0.01, "{fps}");
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "파일 없음"
    );
}

#[test]
fn start_middle_end_and_past_end() {
    let (_d, src) = ramp();
    let (name, size, level) = shot(&src, json!({}));
    assert_eq!(name, "ramp_00m00s000.jpg");
    assert_eq!(size, (320, 240));
    assert!(level < 16.0, "0초 = 검정: {level}");

    let (name, _, level) = shot(&src, json!({ "time": 2.5 }));
    assert_eq!(name, "ramp_00m02s500.jpg");
    let want = ramp_level(4, 2.5);
    assert!((level - want).abs() < 16.0, "{level}, 기대 {want}");

    // 마지막 프레임과 길이를 넘은 시각은 모두 마지막 프레임 (이름표는 영상 길이까지)
    let last = ramp_level(4, 4.0);
    let (_, _, level) = shot(&src, json!({ "time": 59.0 / 15.0 }));
    assert!((level - last).abs() < 16.0, "{level}, 기대 {last}");
    let (name, _, level) = shot(&src, json!({ "time": 100.0 }));
    assert!(name.starts_with("ramp_00m04s"), "{name}");
    assert!((level - last).abs() < 16.0, "{level}, 기대 {last}");
}

/// `k/15`초를 고르면 정확히 k번째 프레임 (바로 앞·뒤 프레임이 아니라)
#[test]
fn picks_exact_frame() {
    let (_d, src) = ramp();
    // make_ramp의 k번째 프레임 밝기 (Y는 정수로 버림)
    let level_of = |k: u32| f64::from(k * 219 / 59) * 255.0 / 219.0;
    for k in [5u32, 14, 15, 31, 45] {
        let t = f64::from(k) / 15.0;
        let (_, _, level) = shot(&src, json!({ "time": t, "format": "png" }));
        let near = |j: u32| (level - level_of(j)).abs();
        assert!(
            near(k) < near(k - 1) && near(k) < near(k + 1),
            "{k}번째: {level}, 기대 {}",
            level_of(k)
        );
    }
}

#[test]
fn png_and_width() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("wide.mov");
    make_video(&src, 1280, 720, 2, false);
    let (name, size, _) = shot(
        &src,
        json!({ "time": 1.5, "format": "png", "width": "w640" }),
    );
    assert_eq!(name, "wide_00m01s500.png");
    assert_eq!(size, (640, 360));
}

#[test]
fn label_times() {
    assert_eq!(label(75.0, false), "01m15s");
    assert_eq!(label(3723.5, false), "1h02m03s");
    assert_eq!(label(1.25, true), "00m01s250");
}

#[test]
fn schema_flags() {
    let s = tool().options_schema();
    assert_eq!(s["x-single-input"], true);
    assert_eq!(s["x-preview"], "result");
    assert_eq!(s["x-seek"], "time");
    assert_eq!(s["x-allow-larger"], true);
    assert_eq!(s["properties"]["time"]["minimum"], 0.0);
}

#[test]
fn not_a_video_is_korean_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("fake.mp4");
    std::fs::write(&src, b"hello").unwrap();
    let e = run(&src, json!({})).err().unwrap();
    assert!(e.to_string().contains("영상"), "{e}");
}

#[test]
fn cancel_stops() {
    let (_d, src) = ramp();
    let dir = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(matches!(e, ToolError::Cancelled));
}

#[test]
fn rotated_video_is_upright() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("portrait.mp4");
    make_marked(&src, 320, 240, 270);
    let (names, out) = run(&src, json!({ "time": 0.5, "format": "png" })).unwrap();
    let img = image::open(out.path().join(&names[0])).unwrap().to_rgb8();
    assert_eq!(img.dimensions(), (240, 320));
    // 회전 270: 저장된 왼쪽 위 구석이 왼쪽 아래로
    assert!(img.get_pixel(10, 310)[0] > 200);
    assert!(img.get_pixel(10, 10)[0] < 50);
}
