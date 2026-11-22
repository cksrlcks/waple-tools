//! OCR 추론 (PaddleOCR 방식 그대로): PP-OCRv5 server det(DB, 글자 영역) → korean PP-OCRv5 mobile rec(CTC, 글자).
//! 모델은 DLL 안에 들어 있고 CPU로만 돌린다. 세션은 처음 쓸 때 만들어 계속 쓴다(ai-common).

use image::RgbImage;
use image::imageops::{FilterType, crop_imm, resize};
use waple_tools_ai_common::{Device, Model, det};
use waple_tools_core::{JobContext, ToolError};

use crate::layout::{Line, Rect};

#[cfg(has_model)]
const DET: &[u8] = include_bytes!("../model/det.onnx");
#[cfg(not(has_model))]
const DET: &[u8] = &[];
#[cfg(has_model)]
const REC: &[u8] = include_bytes!("../model/rec.onnx");
#[cfg(not(has_model))]
const REC: &[u8] = &[];
/// rec 출력 번호 → 글자 (0번은 CTC 빈칸, 맨 끝은 띄어쓰기)
const DICT: &str = include_str!("../model/dict.txt");

// 입력 모양은 GPU 확인용이라 CPU 모델에서는 쓰이지 않는다. 실제 모양은 이미지마다 정한다
static DET_MODEL: Model = Model::new(DET, [1, 3, 32, 32], Device::Cpu);
static REC_MODEL: Model = Model::new(REC, [1, 3, REC_H as usize, 320], Device::Cpu);

/// rec 입력 높이·최소 폭 (PaddleOCR rec_image_shape 3×48×320)
const REC_H: u32 = 48;
const REC_MIN_W: u32 = 320;
const REC_MAX_W: u32 = 3200;
/// 이보다 낮은 인식 결과는 글자가 아닌 그림으로 보고 버린다
pub const DROP_SCORE: f32 = 0.4;

/// 이미지에서 글자 줄 조각(det 상자 하나 = 조각 하나)을 찾아 읽는다
pub fn recognize(ctx: &JobContext, img: &RgbImage) -> Result<Vec<Line>, ToolError> {
    ctx.check_cancel()?;
    let rects: Vec<Rect> = det::detect(&DET_MODEL, img)?
        .into_iter()
        .map(|b| Rect {
            x0: b.x0,
            y0: b.y0,
            x1: b.x1,
            y1: b.y1,
        })
        .collect();
    let dict: Vec<&str> = DICT.lines().map(|l| l.trim_end_matches('\r')).collect();
    REC_MODEL.with_session(|r| {
        let mut out = Vec::new();
        for rect in rects {
            ctx.check_cancel()?;
            let (x, y) = (rect.x0.floor() as u32, rect.y0.floor() as u32);
            let (w, h) = (
                (rect.x1.ceil() as u32).saturating_sub(x),
                (rect.y1.ceil() as u32).saturating_sub(y),
            );
            if w < 2 || h < 2 {
                continue;
            }
            // shortcut: 세로쓰기(높이가 폭보다 큰 상자)는 돌리지 않고 그대로 읽는다. 세로 글자 배너가 필요하면 90° 회전 추가
            let crop = crop_imm(img, x, y, w, h).to_image();
            let (input, pw) = rec_input(&crop);
            let (shape, probs) = r.infer_shaped(vec![1, 3, REC_H as usize, pw], input)?;
            let (t, c) = match shape[..] {
                [1, t, c] if t * c == probs.len() => (t, c),
                _ => return Err(mismatch()),
            };
            let (text, confidence) = ctc_decode(&probs, t, c, &dict)?;
            let text = text.trim().to_string();
            if !text.is_empty() && confidence >= DROP_SCORE {
                out.push(Line {
                    text,
                    confidence,
                    rect,
                });
            }
        }
        Ok(out)
    })
}

fn mismatch() -> ToolError {
    ToolError::Failed("OCR 모델의 출력 모양이 예상과 다릅니다.".into())
}

/// rec 입력: 높이 48로 비율 유지 리사이즈, 폭은 최소 320까지 0(회색)으로 채운다. (텐서, 채운 폭)
fn rec_input(crop: &RgbImage) -> (Vec<f32>, usize) {
    let ratio = crop.width() as f32 / crop.height() as f32;
    let rw = ((REC_H as f32 * ratio).ceil() as u32).clamp(1, REC_MAX_W);
    let pw = rw.max(REC_MIN_W) as usize;
    let small = resize(crop, rw, REC_H, FilterType::Triangle);
    let plane = REC_H as usize * pw;
    let mut t = vec![0f32; 3 * plane];
    for (x, y, p) in small.enumerate_pixels() {
        let [r, g, b] = p.0;
        let i = y as usize * pw + x as usize;
        for (c, v) in [b, g, r].into_iter().enumerate() {
            t[c * plane + i] = f32::from(v) / 255.0 * 2.0 - 1.0;
        }
    }
    (t, pw)
}

/// CTC 탐욕 디코딩: 단계마다 가장 높은 번호, 같은 번호 반복·빈칸(0)은 뺀다. (글, 남긴 글자 확률 평균)
pub(crate) fn ctc_decode(
    probs: &[f32],
    t: usize,
    c: usize,
    dict: &[&str],
) -> Result<(String, f32), ToolError> {
    // 빈칸 + 사전 + 띄어쓰기
    if c != dict.len() + 2 {
        return Err(ToolError::Failed(
            "OCR 글자 사전과 모델이 맞지 않습니다.".into(),
        ));
    }
    let (mut text, mut sum, mut n, mut prev) = (String::new(), 0f32, 0usize, 0usize);
    for step in probs.chunks_exact(c).take(t) {
        let (k, p) = step
            .iter()
            .copied()
            .enumerate()
            .fold((0, f32::MIN), |a, b| if b.1 > a.1 { b } else { a });
        if k != 0 && k != prev {
            text.push_str(dict.get(k - 1).copied().unwrap_or(" "));
            sum += p;
            n += 1;
        }
        prev = k;
    }
    Ok((text, if n == 0 { 0.0 } else { sum / n as f32 }))
}
