use image::imageops::{FilterType, resize};
use image::{Rgb, RgbImage};
use waple_tools_core::ToolError;
use waple_tools_image_common::testkit::photo;

use crate::tile::{MODEL_SCALE, PAD, TILE, spans, upscale_rgb};
use crate::*;

#[test]
fn spans_cover_every_pixel_with_context_at_seams() {
    for len in 1..=1200 {
        let s = spans(len);
        let padded = len.max(TILE);
        assert_eq!(s.first().unwrap().1, 0);
        assert_eq!(s.last().unwrap().2, len);
        for (i, &(start, from, to)) in s.iter().enumerate() {
            assert!(start + TILE <= padded, "len {len}: 창이 이미지 밖");
            assert!(from >= start && to <= start + TILE && from <= to);
            if i > 0 {
                assert_eq!(from, s[i - 1].2, "len {len}: 이어져야 함");
                assert!(from - start >= PAD, "len {len}: 왼쪽 문맥 부족");
            }
            if i + 1 < s.len() {
                assert!(start + TILE - to >= PAD, "len {len}: 오른쪽 문맥 부족");
            }
        }
    }
    assert_eq!(spans(1000).len(), 10);
}

/// 가짜 모델: 최근접 4배. 타일을 어떻게 나눠도 전체를 한 번에 키운 것과 같아야 한다
fn nearest_x4(input: &[f32]) -> Result<Vec<f32>, ToolError> {
    let (t, b) = (TILE as usize, (TILE * MODEL_SCALE) as usize);
    let mut out = vec![0f32; 3 * b * b];
    for c in 0..3 {
        for y in 0..b {
            for x in 0..b {
                out[c * b * b + y * b + x] = input[c * t * t + (y / 4) * t + x / 4];
            }
        }
    }
    Ok(out)
}

#[test]
fn tiles_join_without_seams() {
    for (w, h) in [(300, 200), (50, 30), (128, 128), (129, 251)] {
        let img = photo(w, h).to_rgb8();
        let got = upscale_rgb(&img, 4, nearest_x4, |_, _| Ok(())).unwrap();
        assert_eq!(
            got,
            resize(&img, w * 4, h * 4, FilterType::Nearest),
            "{w}x{h}"
        );
    }
}

#[test]
fn x2_matches_shrinking_the_whole_x4() {
    let img = photo(300, 200).to_rgb8();
    let x4 = resize(&img, 1200, 800, FilterType::Nearest);
    let want = resize(&x4, 600, 400, FilterType::Lanczos3);
    let got = upscale_rgb(&img, 2, nearest_x4, |_, _| Ok(())).unwrap();
    assert_eq!(got.dimensions(), (600, 400));
    let worst = got
        .pixels()
        .zip(want.pixels())
        .flat_map(|(a, b)| (0..3).map(move |c| a[c].abs_diff(b[c])))
        .max()
        .unwrap();
    assert!(worst <= 1, "최대 차이 {worst}");
}

#[test]
fn tile_callback_can_cancel() {
    let img = RgbImage::from_pixel(300, 300, Rgb([1, 2, 3]));
    let mut calls = 0;
    let r = upscale_rgb(
        &img,
        4,
        |t| {
            calls += 1;
            nearest_x4(t)
        },
        |done, total| {
            assert_eq!(total, 9);
            if done == 2 {
                Err(ToolError::Cancelled)
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(r, Err(ToolError::Cancelled)));
    assert_eq!(calls, 2);
}

#[test]
fn wrong_model_output_size_fails() {
    let img = RgbImage::new(10, 10);
    let r = upscale_rgb(&img, 4, |_| Ok(vec![0.0; 3]), |_, _| Ok(()));
    assert!(matches!(r, Err(ToolError::Failed(_))));
}

#[test]
fn output_size_limits() {
    assert_eq!(out_size(100, 50, 4).unwrap(), (400, 200));
    assert_eq!(out_size(4096, 100, 4).unwrap(), (16384, 400));
    assert!(matches!(out_size(4097, 100, 4), Err(ToolError::Failed(m)) if m.contains("16384")));
    assert!(matches!(out_size(100, 8193, 2), Err(ToolError::Failed(_))));
    // 긴 변은 괜찮아도 화소 수(1억) 초과
    assert!(out_size(5001, 5001, 2).is_err());
    assert!(out_size(u32::MAX, 1, 4).is_err());
}

#[test]
fn schema_defaults_and_flags() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "none");
    assert_eq!(s["x-allow-larger"], true);
    let o: UpscaleOptions = parse_options(&serde_json::Value::Null).unwrap();
    assert_eq!(o.scale, Scale::X4);
    let o: UpscaleOptions = parse_options(&serde_json::json!({ "scale": "x2" })).unwrap();
    assert_eq!(o.scale, Scale::X2);
}

#[cfg(not(has_model))]
#[test]
fn without_model_reports_missing_model() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    photo(8, 8).save(&src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = tool().run(&ctx, &[src], dir.path(), &serde_json::Value::Null);
    assert!(matches!(r, Err(ToolError::Failed(m)) if m.contains("AI 모델이 들어 있지 않습니다")));
}

/// 실제 모델로 추론 (model/model.onnx가 있을 때만)
#[cfg(has_model)]
mod with_model {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use waple_tools_core::{JobContext, ToolError, ToolOutput};
    use waple_tools_image_common::testkit::alpha_png;

    use crate::tool;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/red-ball.jpg")
    }

    fn run(input: &Path, out: &Path, scale: &str) -> Result<PathBuf, ToolError> {
        let ctx = JobContext::new(|_| {}, Default::default());
        let opts = serde_json::json!({ "scale": scale });
        match tool().run(&ctx, &[input.to_path_buf()], out, &opts)? {
            ToolOutput::Files(f) => Ok(out.join(&f[0])),
            ToolOutput::Blocks(_) => panic!("파일이어야 함"),
        }
    }

    #[test]
    fn jpeg_x4_keeps_format_colors_and_original() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("ball.jpg");
        std::fs::copy(fixture(), &src).unwrap();
        let before = std::fs::read(&src).unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        let path = run(&src, &out, "x4").unwrap();
        assert_eq!(path.file_name().unwrap(), "ball.jpg");
        let img = image::open(&path).unwrap().to_rgb8();
        assert_eq!(img.dimensions(), (800, 600));
        // 색 순서·정규화 확인: 공 가운데는 빨강, 구석은 밝은 회색
        let ball = img.get_pixel(400, 340);
        assert!(ball[0] > 150 && ball[1] < 110 && ball[2] < 110, "{ball:?}");
        let corner = img.get_pixel(20, 20);
        assert!(corner.0.iter().all(|&c| c > 190), "{corner:?}");
        assert_eq!(std::fs::read(&src).unwrap(), before, "원본 그대로");
    }

    #[test]
    fn alpha_png_x2_keeps_alpha() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.png");
        alpha_png(&src, 40, 30);
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let path = run(&src, &out, "x2").unwrap();
        let img = image::open(&path).unwrap();
        assert!(img.color().has_alpha());
        let img = img.to_rgba8();
        assert_eq!(img.dimensions(), (80, 60));
        assert_eq!(img.get_pixel(5, 30)[3], 0, "왼쪽은 투명");
        assert_eq!(img.get_pixel(75, 30)[3], 255, "오른쪽은 불투명");
    }

    #[test]
    fn not_an_image_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.png");
        std::fs::write(&src, b"not an image").unwrap();
        let r = run(&src, dir.path(), "x4");
        assert!(matches!(r, Err(ToolError::Failed(m)) if m == "이미지를 읽을 수 없습니다."));
    }

    #[test]
    fn cancel_stops_between_tiles() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("ball.jpg");
        std::fs::copy(fixture(), &src).unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let f = flag.clone();
        // 첫 타일 진행률이 오면 취소를 누른다
        let ctx = JobContext::new(
            move |r| {
                if r > 0.0 {
                    f.store(true, Ordering::Relaxed)
                }
            },
            flag,
        );
        let r = tool().run(&ctx, &[src], &out, &serde_json::Value::Null);
        assert!(matches!(r, Err(ToolError::Cancelled)));
        assert_eq!(
            std::fs::read_dir(&out).unwrap().count(),
            0,
            "결과를 쓰지 않음"
        );
    }
}
