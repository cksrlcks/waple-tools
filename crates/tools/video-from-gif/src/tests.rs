use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame};
use serde_json::{Value, json};
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

/// 프레임마다 (지연 ms, 색). 왼쪽 절반은 투명
fn make_gif(path: &Path, w: u32, h: u32, frames: &[(u32, [u8; 3])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = GifEncoder::new(file);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for &(ms, [r, g, b]) in frames {
        let img = RgbaImage::from_fn(w, h, |x, _| {
            if x < w / 2 {
                Rgba([0, 0, 0, 0])
            } else {
                Rgba([r, g, b, 255])
            }
        });
        enc.encode_frame(Frame::from_parts(
            img,
            0,
            0,
            Delay::from_numer_denom_ms(ms, 1),
        ))
        .unwrap();
    }
}

const RED_GREEN_BLUE: &[(u32, [u8; 3])] =
    &[(100, [255, 0, 0]), (500, [0, 255, 0]), (400, [0, 0, 255])];

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

/// (시각 100ns, 왼쪽 가운데 화소, 오른쪽 가운데 화소)
type Shot = (i64, [u8; 3], [u8; 3]);

/// 결과 영상의 모든 프레임
fn read_back(out: &Path) -> (mf::Probe, Vec<Shot>) {
    let p = mf::probe(out).unwrap();
    let mut r = mf::FrameReader::open(out, &p, p.width, p.height).unwrap();
    let px = |rgba: &[u8], x: u32| {
        let i = ((p.height / 2 * p.width + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2]]
    };
    let mut frames = vec![];
    while let Some(f) = r.next_frame().unwrap() {
        let img = r.image(&f).unwrap();
        frames.push((
            f.time,
            px(img.as_raw(), 2),
            px(img.as_raw(), p.width * 3 / 4),
        ));
    }
    (p, frames)
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) < 40)
}

#[test]
fn variable_delays_and_white_background() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("anim.gif");
    make_gif(&src, 120, 80, RED_GREEN_BLUE);
    let (name, out, _o) = convert(&src, json!({})).unwrap();
    assert_eq!(name, PathBuf::from("anim.mp4"));
    let (p, frames) = read_back(&out);
    assert_eq!((p.width, p.height), (120, 80));
    assert!(
        (p.duration as f64 / 1e7 - 1.0).abs() < 0.1,
        "{}",
        p.duration
    );
    assert_eq!(frames.len(), 3, "GIF 프레임 하나 = 영상 프레임 하나");
    let times: Vec<i64> = frames.iter().map(|f| f.0).collect();
    assert_eq!(times, vec![0, 1_000_000, 6_000_000], "지연 0.1초 · 0.5초");
    assert!(
        near(frames[0].1, [255, 255, 255]),
        "투명 → 흰색 {:?}",
        frames[0].1
    );
    assert!(near(frames[0].2, [255, 0, 0]), "{:?}", frames[0].2);
    assert!(near(frames[1].2, [0, 255, 0]), "{:?}", frames[1].2);
    assert!(near(frames[2].2, [0, 0, 255]), "{:?}", frames[2].2);
}

#[test]
fn black_background() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("anim.gif");
    make_gif(&src, 120, 80, RED_GREEN_BLUE);
    let (_, out, _o) = convert(&src, json!({ "background": "black" })).unwrap();
    let (_, frames) = read_back(&out);
    assert!(near(frames[0].1, [0, 0, 0]), "{:?}", frames[0].1);
}

#[test]
fn odd_size_becomes_even() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("odd.gif");
    make_gif(&src, 101, 75, RED_GREEN_BLUE);
    let (_, out, _o) = convert(&src, json!({})).unwrap();
    let p = mf::probe(&out).unwrap();
    assert_eq!((p.width, p.height), (102, 76));
}

#[test]
fn tiny_gif_is_scaled_up() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("tiny.gif");
    make_gif(&src, 16, 12, RED_GREEN_BLUE);
    let (_, out, _o) = convert(&src, json!({})).unwrap();
    let p = mf::probe(&out).unwrap();
    // 짧은 변 12 → 4배 = 48
    assert_eq!((p.width, p.height), (64, 48));
}

#[test]
fn layout_rules() {
    assert_eq!(layout(120, 80), (1, 120, 80));
    assert_eq!(layout(101, 75), (1, 102, 76));
    assert_eq!(layout(16, 12), (4, 64, 48));
    assert_eq!(layout(25, 100), (2, 50, 200));
}

#[test]
fn still_gif_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("still.gif");
    make_gif(&src, 64, 64, &[(100, [255, 0, 0])]);
    let e = convert(&src, json!({})).unwrap_err();
    assert!(e.to_string().contains("움직이지 않는"), "{e}");
}

#[test]
fn not_a_gif_is_korean_error() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("fake.gif");
    std::fs::write(&src, b"hello").unwrap();
    let e = convert(&src, json!({})).unwrap_err();
    assert!(e.to_string().contains("GIF 파일이 아닙니다"), "{e}");
}

#[test]
fn cancel_stops() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("anim.gif");
    make_gif(&src, 120, 80, RED_GREEN_BLUE);
    let out = tempfile::tempdir().unwrap();
    let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
    let e = tool()
        .run(&ctx, &[src], out.path(), &json!({}))
        .unwrap_err();
    assert!(matches!(e, ToolError::Cancelled));
}
