//! 전처리(1024×1024, ImageNet 정규화)와 마스크 후처리(시그모이드, 원본 크기, 알파 적용)

use image::imageops::{FilterType, resize};
use image::{GrayImage, Luma, RgbImage, RgbaImage};
use waple_tools_core::ToolError;

/// 모델 입력·출력 한 변 (preprocessor_config.json: 1024×1024, bilinear)
pub const SIDE: u32 = 1024;
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// RGB → 1024×1024로 늘려 NCHW, `(x/255 - mean) / std`
pub fn to_input(rgb: &RgbImage) -> Vec<f32> {
    let small = resize(rgb, SIDE, SIDE, FilterType::Triangle);
    let plane = (SIDE * SIDE) as usize;
    let mut out = vec![0f32; 3 * plane];
    for (i, p) in small.pixels().enumerate() {
        for c in 0..3 {
            out[c * plane + i] = (f32::from(p[c]) / 255.0 - MEAN[c]) / STD[c];
        }
    }
    out
}

/// 모델 출력(로짓) → 시그모이드 → 원본 크기 마스크 (255 = 남김)
pub fn to_mask(logits: &[f32], w: u32, h: u32) -> Result<GrayImage, ToolError> {
    if logits.len() != (SIDE * SIDE) as usize {
        return Err(ToolError::Failed(
            "AI 모델 결과 크기가 올바르지 않습니다.".into(),
        ));
    }
    let m = GrayImage::from_fn(SIDE, SIDE, |x, y| {
        let v = logits[(y * SIDE + x) as usize];
        Luma([(255.0 / (1.0 + (-v).exp())).round() as u8])
    });
    Ok(resize(&m, w, h, FilterType::Triangle))
}

/// 알파 = 원래 알파 × 마스크
pub fn cut_out(img: &RgbaImage, mask: &GrayImage) -> RgbaImage {
    let mut out = img.clone();
    for (p, m) in out.pixels_mut().zip(mask.pixels()) {
        p[3] = ((u16::from(p[3]) * u16::from(m[0]) + 127) / 255) as u8;
    }
    out
}
