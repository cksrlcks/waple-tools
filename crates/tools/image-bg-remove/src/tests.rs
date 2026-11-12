use image::{GrayImage, Luma, Rgb, RgbImage, Rgba, RgbaImage};
use waple_tools_core::ToolError;

use crate::mask::{SIDE, cut_out, to_input, to_mask};
use crate::*;

#[test]
fn input_is_planar_and_imagenet_normalized() {
    let img = RgbImage::from_pixel(7, 5, Rgb([255, 0, 128]));
    let x = to_input(&img);
    let plane = (SIDE * SIDE) as usize;
    assert_eq!(x.len(), 3 * plane);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(near(x[0], (1.0 - 0.485) / 0.229), "R");
    assert!(near(x[plane + 123], (0.0 - 0.456) / 0.224), "G");
    assert!(
        near(x[2 * plane + plane - 1], (128.0 / 255.0 - 0.406) / 0.225),
        "B"
    );
}

#[test]
fn mask_applies_sigmoid_and_resizes() {
    let n = (SIDE * SIDE) as usize;
    // 왼쪽 절반 로짓 -20(지움), 오른쪽 +20(남김)
    let logits: Vec<f32> = (0..n)
        .map(|i| {
            if (i as u32 % SIDE) < SIDE / 2 {
                -20.0
            } else {
                20.0
            }
        })
        .collect();
    let m = to_mask(&logits, 300, 100).unwrap();
    assert_eq!(m.dimensions(), (300, 100));
    assert_eq!(m.get_pixel(10, 50)[0], 0);
    assert_eq!(m.get_pixel(290, 50)[0], 255);
    let half = to_mask(&vec![0.0; n], 3, 3).unwrap();
    assert_eq!(half.get_pixel(1, 1)[0], 128, "로짓 0 = 0.5");
    assert!(matches!(
        to_mask(&[0.0; 4], 3, 3),
        Err(ToolError::Failed(_))
    ));
}

#[test]
fn cut_out_multiplies_existing_alpha() {
    let img = RgbaImage::from_fn(3, 1, |x, _| Rgba([10, 20, 30, [255, 128, 0][x as usize]]));
    let m = GrayImage::from_pixel(3, 1, Luma([128]));
    let out = cut_out(&img, &m);
    assert_eq!(out.get_pixel(0, 0).0, [10, 20, 30, 128]);
    assert_eq!(out.get_pixel(1, 0)[3], 64);
    assert_eq!(out.get_pixel(2, 0)[3], 0);
}

#[test]
fn schema_and_options() {
    let s = tool().options_schema();
    assert_eq!(s["x-preview"], "original");
    assert_eq!(s["x-single-input"], true);
    assert_eq!(s["x-allow-larger"], true);
    assert_eq!(s["properties"]["color"]["format"], "color");
    let o: BgRemoveOptions = parse_options(&serde_json::Value::Null).unwrap();
    assert_eq!(o.background, Background::Transparent);
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = tool().run(
        &ctx,
        &[],
        Path::new("."),
        &serde_json::json!({ "background": "color", "color": "white" }),
    );
    assert!(matches!(r, Err(ToolError::InvalidOptions(_))));
}

#[cfg(not(has_model))]
#[test]
fn without_model_reports_missing_model() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    RgbImage::new(8, 8).save(&src).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = tool().run(&ctx, &[src], dir.path(), &serde_json::Value::Null);
    assert!(matches!(r, Err(ToolError::Failed(m)) if m.contains("AI 모델이 들어 있지 않습니다")));
}

/// 실제 모델로 추론 (model/model.onnx가 있을 때만)
#[cfg(has_model)]
mod with_model {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use waple_tools_core::{JobContext, ToolError, ToolOutput};

    use crate::tool;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/red-ball.jpg")
    }

    /// 임시 폴더에 샘플을 복사해 실행. (결과 경로, 원본 경로, 원본 바이트, 임시 폴더)
    fn run(opts: serde_json::Value) -> (PathBuf, PathBuf, Vec<u8>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("ball.jpg");
        std::fs::copy(fixture(), &src).unwrap();
        let before = std::fs::read(&src).unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        let ToolOutput::Files(f) = tool()
            .run(&ctx, std::slice::from_ref(&src), &out, &opts)
            .unwrap()
        else {
            panic!("파일이어야 함");
        };
        (out.join(&f[0]), src, before, dir)
    }

    #[test]
    fn transparent_png_keeps_subject_and_original() {
        let (path, src, before, _dir) = run(serde_json::Value::Null);
        assert_eq!(path.file_name().unwrap(), "ball.png");
        let img = image::open(&path).unwrap();
        assert!(img.color().has_alpha());
        let img = img.to_rgba8();
        assert_eq!(img.dimensions(), (200, 150));
        let center = img.get_pixel(100, 78);
        assert!(center[3] > 200 && center[0] > 120, "공은 남김: {center:?}");
        assert!(
            img.get_pixel(5, 5)[3] < 30,
            "배경은 투명: {:?}",
            img.get_pixel(5, 5)
        );
        assert_eq!(std::fs::read(&src).unwrap(), before, "원본 그대로");
    }

    #[test]
    fn color_background_and_mask() {
        let (path, _, _, _dir) =
            run(serde_json::json!({ "background": "color", "color": "#000000" }));
        let img = image::open(&path).unwrap();
        assert!(!img.color().has_alpha());
        assert!(
            img.to_rgb8().get_pixel(5, 5).0.iter().all(|&c| c < 30),
            "검정 배경"
        );

        let (path, _, _, _dir2) = run(serde_json::json!({ "background": "mask" }));
        let img = image::open(&path).unwrap();
        assert_eq!(img.color(), image::ColorType::L8);
        let img = img.to_luma8();
        assert!(img.get_pixel(100, 78)[0] > 200 && img.get_pixel(5, 5)[0] < 30);
    }

    /// 배경 제거는 GPU를 시도하지 않고 늘 CPU 세션을 쓴다
    #[test]
    fn shared_session_is_always_cpu() {
        use waple_tools_ai_common::Device;

        use crate::MODEL;

        MODEL.with_session(|_| Ok(())).unwrap();
        assert_eq!(MODEL.device(), Some(Device::Cpu));
    }

    #[test]
    fn not_an_image_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.png");
        std::fs::write(&src, b"not an image").unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        let r = tool().run(&ctx, &[src], dir.path(), &serde_json::Value::Null);
        assert!(matches!(r, Err(ToolError::Failed(m)) if m == "이미지를 읽을 수 없습니다."));
    }

    #[test]
    fn cancelled_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
        let r = tool().run(&ctx, &[fixture()], dir.path(), &serde_json::Value::Null);
        assert!(matches!(r, Err(ToolError::Cancelled)));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
