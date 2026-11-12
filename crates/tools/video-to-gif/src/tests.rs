use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};
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

struct Gif {
    name: PathBuf,
    bytes: Vec<u8>,
    size: (u32, u32),
    /// (지연 ms, 위 절반 왼쪽 위 화소 밝기)
    frames: Vec<(u32, u8)>,
}

impl Gif {
    fn total_ms(&self) -> u32 {
        self.frames.iter().map(|f| f.0).sum()
    }
}

fn run(src: &Path, opts: Value) -> Result<Gif, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let before = std::fs::read(src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &[src.to_path_buf()], dir.path(), &opts)?
    else {
        panic!("파일이어야 함");
    };
    assert_eq!(std::fs::read(src).unwrap(), before, "원본 그대로");
    let bytes = std::fs::read(dir.path().join(&names[0])).unwrap();
    let dec = GifDecoder::new(Cursor::new(&bytes)).unwrap();
    let size = dec.dimensions();
    let frames = dec
        .into_frames()
        .map(|f| {
            let f = f.unwrap();
            let (n, d) = f.delay().numer_denom_ms();
            (n / d, f.buffer().get_pixel(4, 4)[0])
        })
        .collect();
    Ok(Gif {
        name: names[0].clone(),
        bytes,
        size,
        frames,
    })
}

fn loops(bytes: &[u8]) -> bool {
    bytes.windows(11).any(|w| w == b"NETSCAPE2.0")
}

#[test]
fn defaults_make_looping_480_wide_gif() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("clip.mov");
    make_video(&src, 640, 360, 1, true);
    let g = run(&src, json!({})).unwrap();
    assert_eq!(g.name, PathBuf::from("clip.gif"));
    assert_eq!(g.size, (480, 270));
    assert!(loops(&g.bytes));
    // 1초짜리 영상이라 기본 5초 대신 1초, 초당 10프레임
    assert_eq!(g.total_ms(), 1000);
    assert_eq!(g.frames.len(), 10);
}

#[test]
fn start_length_fps_width_are_respected() {
    let (_d, src) = ramp();
    let g = run(
        &src,
        json!({ "start": 2.0, "length": 1.0, "fps": "f20", "width": "w320", "loop": false }),
    )
    .unwrap();
    assert_eq!(g.size, (320, 240));
    assert!(!loops(&g.bytes), "한 번만 재생");
    // 원본은 15fps라 20fps 목표 중 일부는 같은 프레임 → 합쳐진다. 재생 시간은 정확히 1초
    assert_eq!(g.total_ms(), 1000);
    assert!((10..=16).contains(&g.frames.len()), "{}", g.frames.len());
    let first = f64::from(g.frames[0].1);
    let want = ramp_level(4, 2.0);
    assert!(
        (first - want).abs() < 16.0,
        "첫 프레임 밝기 {first}, 기대 {want}"
    );
    let last = f64::from(g.frames.last().unwrap().1);
    assert!(last > first + 30.0, "시간 따라 밝아짐: {first} → {last}");
}

#[test]
fn smaller_width_scales_down() {
    let (_d, src) = ramp();
    let g = run(
        &src,
        json!({ "length": 0.5, "width": "w320", "fps": "f10" }),
    )
    .unwrap();
    assert_eq!(g.size, (320, 240));
    let dir = tempfile::tempdir().unwrap();
    let wide = dir.path().join("wide.mp4");
    make_video(&wide, 1280, 720, 1, false);
    let g = run(&wide, json!({ "length": 0.5, "width": "w320" })).unwrap();
    assert_eq!(g.size, (320, 180));
    assert_eq!(g.total_ms(), 500);
}

#[test]
fn uneven_frame_times_keep_their_delays() {
    // 화면 녹화처럼 프레임 간격이 다른 영상: 빨강 0.1초 · 초록 0.5초 · 파랑 0.4초
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("vfr.mp4");
    let wr = mf::Mp4Writer::create(&src, 64, 48, (30, 1), 1_000_000).unwrap();
    let mut t = 0;
    for (ms, c) in [(100, [255, 0, 0]), (500, [0, 255, 0]), (400, [0, 0, 255])] {
        let img = image::RgbaImage::from_pixel(64, 48, image::Rgba([c[0], c[1], c[2], 255]));
        wr.write(img.as_raw(), t, ms * 10_000).unwrap();
        t += ms * 10_000;
    }
    wr.finish().unwrap();
    let g = run(&src, json!({ "width": "original" })).unwrap();
    let delays: Vec<u32> = g.frames.iter().map(|f| f.0).collect();
    assert_eq!(delays, vec![100, 500, 400]);
}

#[test]
fn start_past_end_is_error() {
    let (_d, src) = ramp();
    let e = run(&src, json!({ "start": 10.0 })).err().unwrap();
    assert!(e.to_string().contains("영상 길이"), "{e}");
    assert!(e.to_string().contains("(10.0초)"), "소수 한 자리: {e}");
}

#[test]
fn too_big_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("big.mp4");
    make_video(&src, 1920, 1080, 3, false);
    // 1920×1080 × 20fps × 3초 ≈ 1.2억 화소 > 상한
    let e = run(
        &src,
        json!({ "width": "original", "fps": "f20", "length": 30 }),
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("너무 큽니다"), "{e}");
}

#[test]
fn delays_add_up_without_drift() {
    for fps in [10, 15, 20] {
        let total: u32 = (0..fps * 3).map(|k| delay_ms(k, fps)).sum();
        assert_eq!(total, 3000, "{fps}fps");
    }
    assert_eq!(delay_ms(0, 15), 70);
    assert_eq!(delay_ms(1, 15), 60);
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
fn rotated_video_becomes_upright_gif() {
    // 휴대폰 세로 영상: 320×240으로 저장, 회전 90 → 240×320 GIF, 흰 구석은 오른쪽 위
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("portrait.mp4");
    make_marked(&src, 320, 240, 90);
    let g = run(&src, json!({ "width": "original", "length": 0.5 })).unwrap();
    assert_eq!(g.size, (240, 320));
    let dec = GifDecoder::new(Cursor::new(&g.bytes)).unwrap();
    let f = dec.into_frames().next().unwrap().unwrap();
    let img = f.buffer();
    assert!(img.get_pixel(230, 10)[0] > 200, "오른쪽 위가 흰색");
    assert!(img.get_pixel(10, 10)[0] < 50, "왼쪽 위는 검정");
}

#[test]
fn schema_allows_larger_result() {
    assert_eq!(tool().options_schema()["x-allow-larger"], true);
}
