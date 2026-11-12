use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_video_common::testkit::make_video;

use super::*;

fn sample(name: &str, audio: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join(name);
    make_video(&src, 320, 240, 1, audio);
    (dir, src)
}

/// 변환해 (결과 이름, 결과 경로, 임시 폴더). 원본이 그대로인지 확인한다
fn convert(src: &Path, opts: Value) -> Result<(PathBuf, PathBuf, tempfile::TempDir), ToolError> {
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

fn secs(p: &mf::Probe) -> f64 {
    p.duration as f64 / 1e7
}

#[test]
fn mov_to_mp4_keeps_size_and_sound() {
    let (_d, src) = sample("clip.mov", true);
    let (name, out, _o) = convert(&src, json!({})).unwrap();
    assert_eq!(name, PathBuf::from("clip.mp4"));
    let p = mf::probe(&out).unwrap();
    assert_eq!((p.width, p.height), (320, 240));
    assert!(p.has_audio);
    assert!((secs(&p) - 1.0).abs() < 0.3, "{}초", secs(&p));
}

#[test]
fn to_wmv() {
    let (_d, src) = sample("clip.mp4", true);
    let (name, out, _o) = convert(&src, json!({ "format": "wmv" })).unwrap();
    assert_eq!(name, PathBuf::from("clip.wmv"));
    let head = std::fs::read(&out).unwrap();
    // ASF 머리 GUID 30 26 B2 75
    assert_eq!(&head[..4], &[0x30, 0x26, 0xB2, 0x75]);
    let p = mf::probe(&out).unwrap();
    assert_eq!((p.width, p.height), (320, 240));
    assert!(p.has_audio);
}

#[test]
fn audio_off_removes_sound() {
    let (_d, src) = sample("clip.mp4", true);
    let (_, out, _o) = convert(&src, json!({ "audio": false })).unwrap();
    assert!(!mf::probe(&out).unwrap().has_audio);
}

/// 소리만 뽑은 파일: Windows가 열 수 있고(영상 트랙만 없다) 머리가 맞는다
fn assert_audio_only(out: &Path, head_ok: impl Fn(&[u8]) -> bool) {
    let bytes = std::fs::read(out).unwrap();
    assert!(bytes.len() > 4_000, "{}바이트", bytes.len());
    assert!(head_ok(&bytes), "머리 {:02X?}", &bytes[..12]);
    let e = mf::probe(out).unwrap_err();
    assert!(e.to_string().contains("영상 트랙이 없는"), "{e}");
}

#[test]
fn to_m4a_extracts_sound() {
    let (_d, src) = sample("clip.mkv.mp4", true);
    let (name, out, _o) = convert(&src, json!({ "format": "m4a" })).unwrap();
    assert_eq!(name, PathBuf::from("clip.mkv.m4a"));
    assert_audio_only(&out, |b| &b[4..8] == b"ftyp");
}

#[test]
fn to_mp3_extracts_sound() {
    let (_d, src) = sample("clip.mp4", true);
    let (name, out, _o) = convert(&src, json!({ "format": "mp3" })).unwrap();
    assert_eq!(name, PathBuf::from("clip.mp3"));
    assert_audio_only(&out, |b| {
        b.starts_with(b"ID3") || (b[0] == 0xFF && b[1] & 0xE0 == 0xE0)
    });
}

#[test]
fn sound_only_without_sound_is_error() {
    let (_d, src) = sample("silent.mp4", false);
    let e = convert(&src, json!({ "format": "mp3" })).unwrap_err();
    assert!(e.to_string().contains("소리가 없는"), "{e}");
}

#[test]
fn not_a_video_is_korean_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("fake.mov");
    std::fs::write(&src, b"hello").unwrap();
    let e = convert(&src, json!({})).unwrap_err();
    assert!(e.to_string().contains("영상"), "{e}");
}

#[test]
fn cancel_stops() {
    let (_d, src) = sample("clip.mp4", false);
    let dir = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let e = tool()
        .run(&ctx, &[src], dir.path(), &json!({}))
        .unwrap_err();
    assert!(matches!(e, ToolError::Cancelled));
}
