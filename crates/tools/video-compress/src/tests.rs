use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_video_common::testkit::make_video;

use super::*;

/// `src`를 줄여 (결과 이름, 결과 경로, 임시 폴더)
fn compress(
    src: &Path,
    opts: serde_json::Value,
) -> Result<(PathBuf, PathBuf, tempfile::TempDir), ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let before = std::fs::read(src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &[src.to_path_buf()], dir.path(), &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(std::fs::read(src).unwrap(), before, "원본 그대로");
    let out = dir.path().join(&names[0]);
    Ok((names[0].clone(), out, dir))
}

fn sample(name: &str, w: u32, h: u32, secs: u32, audio: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join(name);
    make_video(&src, w, h, secs, audio);
    (dir, src)
}

fn secs(p: &mf::Probe) -> f64 {
    p.duration as f64 / 1e7
}

#[test]
fn downscales_to_480p_and_keeps_audio() {
    let (_d, src) = sample("clip.mov", 1280, 720, 2, true);
    let (name, out, _o) = compress(&src, json!({ "resolution": "p480" })).unwrap();
    assert_eq!(name, PathBuf::from("clip.mp4"));
    let p = mf::probe(&out).unwrap();
    assert_eq!((p.width, p.height), (854, 480));
    assert!(p.has_audio);
    assert!((secs(&p) - 2.0).abs() < 0.3, "{}초", secs(&p));
    let (a, b) = (src.metadata().unwrap().len(), out.metadata().unwrap().len());
    assert!(b * 2 < a, "{a} → {b}");
}

#[test]
fn audio_off_removes_sound() {
    let (_d, src) = sample("clip.mp4", 640, 360, 1, true);
    let (_, out, _o) = compress(&src, json!({ "audio": false })).unwrap();
    let p = mf::probe(&out).unwrap();
    assert_eq!((p.width, p.height), (640, 360), "원본 크기");
    assert!(!p.has_audio);
}

#[test]
fn target_size_is_respected() {
    // 원본 8Mbps × 3초 ≈ 3MB → 1MB
    let (_d, src) = sample("clip.mp4", 1280, 720, 3, true);
    let (_, out, _o) = compress(&src, json!({ "quality": "targetMb", "targetMb": 1 })).unwrap();
    let len = out.metadata().unwrap().len();
    assert!(len <= 1024 * 1024 * 11 / 10, "{len}바이트");
    assert!(len >= 1024 * 1024 / 2, "너무 작음: {len}바이트");
}

#[test]
fn not_a_video_is_korean_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("fake.mp4");
    std::fs::write(&src, b"hello").unwrap();
    let e = compress(&src, json!({})).unwrap_err();
    assert!(e.to_string().contains("영상"), "{e}");
}

#[test]
fn cancel_stops() {
    let (_d, src) = sample("clip.mp4", 640, 360, 1, false);
    let dir = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(matches!(e, ToolError::Cancelled));
}

#[test]
fn scaled_keeps_short_side_and_even() {
    assert_eq!(scaled(1280, 720, Some(480)), (854, 480));
    assert_eq!(scaled(1080, 1920, Some(720)), (720, 1280), "세로 영상");
    assert_eq!(scaled(640, 360, Some(1080)), (640, 360), "키우지 않음");
    assert_eq!(scaled(641, 361, None), (640, 360), "짝수");
}

#[test]
fn too_small_target_is_error() {
    // 10분 영상을 1MB로
    let e = video_bitrate(
        Quality::TargetMb,
        1,
        (1280, 720),
        (30, 1),
        600 * 10_000_000,
        true,
    )
    .unwrap_err();
    assert!(e.to_string().contains("너무 작습니다"), "{e}");
}

#[test]
fn quality_levels_order() {
    let b = |q| video_bitrate(q, 0, (1920, 1080), (30, 1), 10_000_000, true).unwrap();
    assert!(b(Quality::High) > b(Quality::Medium));
    assert!(b(Quality::Medium) > b(Quality::Low));
}
