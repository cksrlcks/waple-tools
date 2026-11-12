use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};
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

#[test]
fn count_spreads_evenly() {
    let (_d, src) = ramp();
    let (names, out) = run(&src, json!({ "count": 4 })).unwrap();
    // 4초를 4칸으로: 0.5 · 1.5 · 2.5 · 3.5초
    assert_eq!(
        names,
        vec![
            "ramp/ramp_00m00s.jpg",
            "ramp/ramp_00m01s.jpg",
            "ramp/ramp_00m02s.jpg",
            "ramp/ramp_00m03s.jpg",
        ]
    );
    for (n, t) in names.iter().zip([0.5, 1.5, 2.5, 3.5]) {
        let (size, level) = look(out.path(), n);
        assert_eq!(size, (320, 240));
        let want = ramp_level(4, t);
        assert!((level - want).abs() < 16.0, "{n}: {level}, 기대 {want}");
    }
}

#[test]
fn same_second_names_get_milliseconds() {
    let (_d, src) = ramp();
    let (names, _out) = run(&src, json!({ "count": 8 })).unwrap();
    assert_eq!(names.len(), 8);
    assert_eq!(names[0], "ramp/ramp_00m00s250.jpg");
    assert_eq!(names[1], "ramp/ramp_00m00s750.jpg");
}

#[test]
fn interval_from_zero() {
    let (_d, src) = ramp();
    let (names, out) = run(&src, json!({ "mode": "interval", "interval": 1 })).unwrap();
    assert_eq!(names.len(), 4);
    assert_eq!(names[3], "ramp/ramp_00m03s.jpg");
    let (_, first) = look(out.path(), &names[0]);
    assert!(first < 16.0, "0초 = 검정: {first}");
}

#[test]
fn times_list_png_and_width() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("wide.mov");
    make_video(&src, 1280, 720, 2, false);
    let (names, out) = run(
        &src,
        json!({ "mode": "times", "times": "1.5, 0:01  0", "format": "png", "width": "w640" }),
    )
    .unwrap();
    // 시간 순으로 정렬, 1초·1.5초가 같은 초라 모두 밀리초까지
    assert_eq!(
        names,
        vec![
            "wide/wide_00m00s000.png",
            "wide/wide_00m01s000.png",
            "wide/wide_00m01s500.png",
        ]
    );
    let (size, _) = look(out.path(), &names[0]);
    assert_eq!(size, (640, 360));
}

#[test]
fn bad_times_are_errors() {
    let (_d, src) = ramp();
    let e = run(&src, json!({ "mode": "times", "times": "1:2x" }))
        .err()
        .unwrap();
    assert!(e.to_string().contains("형식"), "{e}");
    let e = run(&src, json!({ "mode": "times", "times": "0:05" }))
        .err()
        .unwrap();
    assert!(e.to_string().contains("영상 길이"), "{e}");
    let e = run(&src, json!({ "mode": "times", "times": " " }))
        .err()
        .unwrap();
    assert!(e.to_string().contains("하나 이상"), "{e}");
}

#[test]
fn shot_limit() {
    let o = ThumbOptions {
        mode: Mode::Interval,
        interval: 1,
        ..Default::default()
    };
    let e = shot_times(&o, Some(500.0)).unwrap_err();
    assert!(e.to_string().contains("최대 100장"), "{e}");
    assert_eq!(shot_times(&o, Some(100.0)).unwrap().len(), 100);
    let o = ThumbOptions {
        count: 1000,
        ..Default::default()
    };
    assert_eq!(shot_times(&o, Some(10.0)).unwrap().len(), MAX_SHOTS);
}

#[test]
fn parse_and_label_times() {
    assert_eq!(parse_time("75"), Some(75.0));
    assert_eq!(parse_time("1:15"), Some(75.0));
    assert_eq!(parse_time("1:02:03.5"), Some(3723.5));
    assert_eq!(parse_time("1:75"), None);
    assert_eq!(parse_time("-3"), None);
    assert_eq!(parse_time("a"), None);
    assert_eq!(label(75.0, false), "01m15s");
    assert_eq!(label(3723.5, false), "1h02m03s");
    assert_eq!(label(1.25, true), "00m01s250");
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
    let (names, out) = run(&src, json!({ "count": 1, "format": "png" })).unwrap();
    let img = image::open(out.path().join(&names[0])).unwrap().to_rgb8();
    assert_eq!(img.dimensions(), (240, 320));
    // 회전 270: 저장된 왼쪽 위 구석이 왼쪽 아래로
    assert!(img.get_pixel(10, 310)[0] > 200);
    assert!(img.get_pixel(10, 10)[0] < 50);
}

#[test]
fn same_millisecond_times_are_one_shot() {
    // 밀리초까지 같은 이름이 둘이면 결과 세트를 저장하지 못한다
    let o = ThumbOptions {
        mode: Mode::Times,
        times: "1.0004 1.0001 1 2".into(),
        ..Default::default()
    };
    let t = shot_times(&o, Some(5.0)).unwrap();
    assert_eq!(t, vec![1.0, 2.0]);
    let o = ThumbOptions {
        count: 100,
        ..Default::default()
    };
    let t = shot_times(&o, Some(0.05)).unwrap();
    let names = labels(&t);
    let mut unique = names.clone();
    unique.dedup();
    assert_eq!(names, unique, "이름이 모두 다름");
}

#[test]
fn unknown_length_allows_only_times() {
    let e = shot_times(&ThumbOptions::default(), None).unwrap_err();
    assert!(e.to_string().contains("지정 시간"), "{e}");
    let o = ThumbOptions {
        mode: Mode::Times,
        times: "3 1:00".into(),
        ..Default::default()
    };
    assert_eq!(shot_times(&o, None).unwrap(), vec![3.0, 60.0]);
}
