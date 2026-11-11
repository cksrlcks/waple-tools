use std::path::{Path, PathBuf};

use image::codecs::gif::{GifEncoder, Repeat as ImgRepeat};
use image::{Delay, Rgba, RgbaImage};
use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::testkit::photo;

use super::*;

/// (프레임, 지연 1/100초)로 움직이는 GIF를 만든다. 프레임마다 전체 크기 · 개별 팔레트 (보통 편집기 출력처럼)
fn make_gif(path: &Path, frames: &[(RgbaImage, u32)], repeat: Option<ImgRepeat>) {
    let mut enc = GifEncoder::new(std::fs::File::create(path).unwrap());
    if let Some(r) = repeat {
        enc.set_repeat(r).unwrap();
    }
    for (img, cs) in frames {
        enc.encode_frame(image::Frame::from_parts(
            img.clone(),
            0,
            0,
            Delay::from_numer_denom_ms(cs * 10, 1),
        ))
        .unwrap();
    }
}

/// 배경 위에서 빨간 8×8 사각형이 오른쪽으로 움직인다
fn moving_square(w: u32, h: u32, n: u32, bg: Rgba<u8>) -> Vec<(RgbaImage, u32)> {
    (0..n)
        .map(|i| {
            let sx = i * 4 % (w - 8);
            let img = RgbaImage::from_fn(w, h, |x, y| {
                if (sx..sx + 8).contains(&x) && (10..18).contains(&y) {
                    Rgba([255, 0, 0, 255])
                } else {
                    bg
                }
            });
            (img, 10)
        })
        .collect()
}

/// 사진 같은 프레임 (색이 많다): 노이즈 이미지를 프레임마다 밀어서
fn photo_frames(w: u32, h: u32, n: u32) -> Vec<(RgbaImage, u32)> {
    let base = photo(w * 2, h).to_rgba8();
    (0..n)
        .map(|i| {
            let img = image::imageops::crop_imm(&base, i * 3, 0, w, h).to_image();
            (img, 5)
        })
        .collect()
}

struct Out {
    frames: Vec<(RgbaImage, u32)>,
    repeat: gif::Repeat,
    palette_len: usize,
    bytes: Vec<u8>,
}

fn read_gif(bytes: &[u8]) -> Out {
    let frames = GifDecoder::new(Cursor::new(bytes))
        .unwrap()
        .into_frames()
        .map(|f| {
            let f = f.unwrap();
            let (n, d) = f.delay().numer_denom_ms();
            (f.into_buffer(), n / d)
        })
        .collect();
    let dec = gif::Decoder::new(Cursor::new(bytes)).unwrap();
    Out {
        frames,
        repeat: dec.repeat(),
        palette_len: dec.global_palette().map_or(0, |p| p.len() / 3),
        bytes: bytes.to_vec(),
    }
}

fn run_file(src: &Path, opts: serde_json::Value) -> Result<Out, ToolError> {
    let out = src.parent().unwrap().join("out");
    std::fs::create_dir_all(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let ToolOutput::Files(names) = tool().run(&ctx, &[src.to_path_buf()], &out, &opts)? else {
        panic!("파일이어야 함");
    };
    assert_eq!(names, [PathBuf::from("a.gif")]);
    Ok(read_gif(&std::fs::read(out.join(&names[0])).unwrap()))
}

/// 반환: (원본 바이트, 결과)
fn run(frames: &[(RgbaImage, u32)], opts: serde_json::Value) -> (Vec<u8>, Out) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.gif");
    make_gif(&src, frames, Some(ImgRepeat::Infinite));
    let orig = std::fs::read(&src).unwrap();
    let out = run_file(&src, opts).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), orig, "원본은 그대로");
    (orig, out)
}

/// 합성된 두 프레임의 채널 평균 오차 (투명 여부는 같아야 한다)
fn mean_diff(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut sum = 0u64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        assert_eq!(p[3] >= 128, q[3] >= 128, "투명 여부가 같아야 함");
        if p[3] >= 128 {
            sum += (0..3).map(|c| u64::from(p[c].abs_diff(q[c]))).sum::<u64>();
        }
    }
    sum as f64 / (a.len() as f64 * 0.75)
}

#[test]
fn frame_step_keeps_total_delay() {
    let frames = moving_square(64, 48, 8, Rgba([255, 255, 255, 255]));
    let (_, out) = run(&frames, json!({ "frameStep": 3 }));
    // 0, 3, 6번이 남고 1·2 → 0, 4·5 → 3, 7 → 6에 시간이 더해진다
    let delays: Vec<u32> = out.frames.iter().map(|f| f.1).collect();
    assert_eq!(delays, [300, 300, 200]);
    for (k, i) in [0usize, 3, 6].into_iter().enumerate() {
        assert_eq!(
            mean_diff(&out.frames[k].0, &frames[i].0),
            0.0,
            "{i}번 프레임"
        );
    }
}

#[test]
fn scale_resizes() {
    let frames = moving_square(64, 48, 4, Rgba([255, 255, 255, 255]));
    let (_, out) = run(&frames, json!({ "scale": 50 }));
    assert_eq!(out.frames.len(), 4);
    assert!(out.frames.iter().all(|f| f.0.dimensions() == (32, 24)));
}

#[test]
fn colors_are_limited() {
    let (_, out) = run(&photo_frames(64, 48, 6), json!({ "colors": 16 }));
    assert_eq!(
        out.palette_len, 32,
        "16색 + 투명 1칸 → 2의 거듭제곱으로 채움"
    );
    let mut seen = std::collections::HashSet::new();
    for (f, _) in &out.frames {
        seen.extend(f.pixels().map(|p| p.0));
    }
    assert!(seen.len() <= 16, "{}색", seen.len());
}

#[test]
fn optimized_frames_look_the_same() {
    // 색이 적으면 원래 색 그대로: 프레임 간 최적화 후에도 화면이 정확히 같아야 한다
    let frames = moving_square(64, 48, 12, Rgba([255, 255, 255, 255]));
    let (orig, out) = run(&frames, json!({}));
    assert_eq!(out.frames.len(), frames.len());
    for (a, b) in out.frames.iter().zip(&frames) {
        assert_eq!(mean_diff(&a.0, &b.0), 0.0);
        assert_eq!(a.1, 100);
    }
    assert!(
        out.bytes.len() < orig.len(),
        "{} → {}",
        orig.len(),
        out.bytes.len()
    );

    // 색이 많으면 양자화 오차만 (노이즈 사진이라 오차가 큰 편. 프레임이 깨지면 수십이 나온다)
    let (orig, out) = run(&photo_frames(64, 48, 6), json!({ "colors": 256 }));
    for (a, b) in out.frames.iter().zip(&read_gif(&orig).frames) {
        let d = mean_diff(&a.0, &b.0);
        assert!(d < 8.0, "평균 오차 {d}");
    }
}

#[test]
fn simple_animation_gets_smaller() {
    let frames = moving_square(200, 150, 30, Rgba([255, 255, 255, 255]));
    let (orig, out) = run(&frames, json!({}));
    assert!(
        out.bytes.len() * 2 < orig.len(),
        "{} → {}",
        orig.len(),
        out.bytes.len()
    );
}

#[test]
fn transparent_gif_keeps_transparency() {
    // 사각형이 지나간 자리는 다시 투명해져야 한다 (앞 화면 유지만으로는 못 지운다)
    let frames = moving_square(64, 48, 10, Rgba([0, 0, 0, 0]));
    let (_, out) = run(&frames, json!({}));
    assert_eq!(out.frames.len(), frames.len());
    for (a, b) in out.frames.iter().zip(&frames) {
        assert_eq!(mean_diff(&a.0, &b.0), 0.0);
    }
}

#[test]
fn static_gif() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.gif");
    photo(80, 60).save(&src).unwrap();
    let out = run_file(&src, json!({ "colors": 64 })).unwrap();
    assert_eq!(out.frames.len(), 1);
    assert_eq!(out.frames[0].0.dimensions(), (80, 60));
    assert_eq!(out.repeat, gif::Repeat::Finite(0), "반복 정보 없음 그대로");
    let orig = image::open(&src).unwrap().to_rgba8();
    assert!(mean_diff(&out.frames[0].0, &orig) < 12.0);
}

#[test]
fn loop_count_is_kept() {
    let frames = moving_square(32, 24, 3, Rgba([255, 255, 255, 255]));
    for (r, want) in [
        (Some(ImgRepeat::Finite(3)), gif::Repeat::Finite(3)),
        (Some(ImgRepeat::Infinite), gif::Repeat::Infinite),
        (None, gif::Repeat::Finite(0)),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.gif");
        make_gif(&src, &frames, r);
        assert_eq!(run_file(&src, json!({})).unwrap().repeat, want);
    }
}

#[test]
fn rejects_non_gif() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.gif");
    photo(8, 8)
        .save_with_format(&src, image::ImageFormat::Png)
        .unwrap();
    let err = run_file(&src, json!({})).err().unwrap();
    assert!(matches!(err, ToolError::Failed(m) if m == "GIF 파일이 아닙니다."));
}

#[test]
fn rejects_too_many_pixels() {
    // 4000×4000 화면에 1×1 프레임 7개: 합성하면 1억 1천만 화소
    let mut bytes = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut bytes, 4000, 4000, &[0, 0, 0, 255, 255, 255]).unwrap();
        for i in 0..7u8 {
            enc.write_frame(&gif::Frame::from_indexed_pixels(1, 1, [i % 2], None))
                .unwrap();
        }
    }
    let ctx = JobContext::new(|_| {}, Default::default());
    let err = optimize(&bytes, &GifOptions::default(), &ctx)
        .err()
        .unwrap();
    assert!(matches!(err, ToolError::Failed(m) if m.contains("1억 화소")));
}

#[test]
fn tiny_delays_play_like_browsers() {
    // 브라우저는 0·1cs를 10cs로 재생한다: 둘씩 합치면 프레임마다 20cs여야 재생 시간이 같다
    for cs in [0, 1] {
        let frames: Vec<_> = moving_square(64, 48, 20, Rgba([255, 255, 255, 255]))
            .into_iter()
            .map(|(img, _)| (img, cs))
            .collect();
        let (_, out) = run(&frames, json!({ "frameStep": 2 }));
        let delays: Vec<u32> = out.frames.iter().map(|f| f.1).collect();
        assert_eq!(delays, [200; 10], "{cs}cs");
    }
}

#[test]
fn two_colors_stay_two() {
    // 흑백 2색: 투명 칸 때문에 한 색으로 뭉개지면 안 된다
    let frames: Vec<_> = (0..4)
        .map(|i| {
            let img = RgbaImage::from_fn(32, 24, |x, y| {
                if (x / 4 + y / 4 + i) % 2 == 0 {
                    Rgba([0, 0, 0, 255])
                } else {
                    Rgba([255, 255, 255, 255])
                }
            });
            (img, 10)
        })
        .collect();
    let (_, out) = run(&frames, json!({ "colors": 2 }));
    for (a, b) in out.frames.iter().zip(&frames) {
        assert_eq!(mean_diff(&a.0, &b.0), 0.0);
    }
}

#[test]
fn transparent_background_frames_stay_small() {
    // 투명 배경 위 움직이는 사각형: 지울 곳만 펴야 프레임이 화면 전체 크기가 되지 않는다
    let frames = moving_square(200, 150, 20, Rgba([0, 0, 0, 0]));
    let (orig, out) = run(&frames, json!({}));
    for (a, b) in out.frames.iter().zip(&frames) {
        assert_eq!(mean_diff(&a.0, &b.0), 0.0);
    }
    let mut dec = gif::Decoder::new(Cursor::new(&out.bytes)).unwrap();
    while let Some(f) = dec.read_next_frame().unwrap() {
        let area = usize::from(f.width) * usize::from(f.height);
        assert!(area <= 12 * 8, "{}×{}", f.width, f.height);
    }
    assert!(
        out.bytes.len() * 2 < orig.len(),
        "{} → {}",
        orig.len(),
        out.bytes.len()
    );
}
