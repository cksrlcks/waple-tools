//! 글자 영역 검출: PaddleOCR PP-OCRv5 mobile det (DLL 안, CPU). 글자를 읽지는 않는다.

use image::RgbImage;
use waple_tools_ai_common::{Device, Model, det};
use waple_tools_core::ToolError;

use crate::analyze::Rect;

#[cfg(has_model)]
const DET: &[u8] = include_bytes!("../model/det.onnx");
#[cfg(not(has_model))]
const DET: &[u8] = &[];

// 입력 모양은 GPU 확인용이라 CPU 모델에서는 쓰이지 않는다. 실제 모양은 캡처마다 정한다
static DET_MODEL: Model = Model::new(DET, [1, 3, 32, 32], Device::Cpu);

/// 모델 없이 빌드되었으면 false
pub(crate) fn available() -> bool {
    !DET.is_empty()
}

/// BGRA 캡처에서 글자 줄 상자
pub(crate) fn text_boxes(px: &[u8], w: usize, h: usize) -> Result<Vec<Rect>, ToolError> {
    let rgb: Vec<u8> = px
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[2], p[1], p[0]])
        .collect();
    let img = RgbImage::from_raw(w as u32, h as u32, rgb)
        .ok_or_else(|| ToolError::Failed("캡처 크기가 맞지 않습니다.".into()))?;
    Ok(det::detect(&DET_MODEL, &img)?
        .into_iter()
        .map(|b| {
            let (x, y) = (b.x0.floor() as i32, b.y0.floor() as i32);
            Rect {
                x,
                y,
                w: b.x1.ceil() as i32 - x,
                h: b.y1.ceil() as i32 - y,
            }
        })
        .collect())
}
