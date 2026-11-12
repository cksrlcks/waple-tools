//! OCR 추론 (PaddleOCR 방식 그대로): PP-OCRv5 server det(DB, 글자 영역) → korean PP-OCRv5 mobile rec(CTC, 글자).
//! 모델은 DLL 안에 들어 있고 CPU로만 돌린다. 세션은 처음 쓸 때 만들어 계속 쓴다(ai-common).

use image::RgbImage;
use image::imageops::{FilterType, crop_imm, resize};
use waple_tools_ai_common::{Device, Model};
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

/// det 입력 긴 변 상한·짧은 변 하한 (PaddleOCR server det 기본값)
const DET_MAX_SIDE: f32 = 4000.0;
const DET_MIN_SIDE: f32 = 64.0;
/// det 입력 화소 수 상한 (CPU 시간). 긴 변 기준으로 줄이면 세로로 긴 공지의 글자가 너무 작아져 넓이로 줄인다
const DET_MAX_PIXELS: f32 = 2_500_000.0;
/// DB 후처리 (PaddleOCR PP-OCRv5 server det 기본값)
const DET_THRESH: f32 = 0.3;
const BOX_THRESH: f32 = 0.6;
const UNCLIP_RATIO: f32 = 1.5;
const MAX_CANDIDATES: usize = 1000;
/// rec 입력 높이·최소 폭 (PaddleOCR rec_image_shape 3×48×320)
const REC_H: u32 = 48;
const REC_MIN_W: u32 = 320;
const REC_MAX_W: u32 = 3200;
/// 이보다 낮은 인식 결과는 글자가 아닌 그림으로 보고 버린다
pub const DROP_SCORE: f32 = 0.4;

/// det 정규화 (ImageNet, BGR 순서로 넣는다: PaddleOCR가 OpenCV BGR 그대로 정규화)
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// 이미지에서 글자 줄 조각(det 상자 하나 = 조각 하나)을 찾아 읽는다
pub fn recognize(ctx: &JobContext, img: &RgbImage) -> Result<Vec<Line>, ToolError> {
    let rects = DET_MODEL.with_session(|r| {
        ctx.check_cancel()?;
        let (rw, rh) = det_size(img.width(), img.height());
        let input = det_input(img, rw, rh);
        let (shape, prob) = r.infer_shaped(vec![1, 3, rh as usize, rw as usize], input)?;
        let (h, w) = match shape[..] {
            [1, 1, h, w] if h * w == prob.len() => (h, w),
            _ => return Err(mismatch()),
        };
        let (sx, sy) = (
            img.width() as f32 / w as f32,
            img.height() as f32 / h as f32,
        );
        Ok(det_boxes(&prob, w, h)
            .into_iter()
            .map(|b| Rect {
                x0: (b.x0 * sx).clamp(0.0, img.width() as f32),
                y0: (b.y0 * sy).clamp(0.0, img.height() as f32),
                x1: (b.x1 * sx).clamp(0.0, img.width() as f32),
                y1: (b.y1 * sy).clamp(0.0, img.height() as f32),
            })
            .collect::<Vec<_>>())
    })?;
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

/// det 입력 크기: 화소 수·긴 변 상한, 짧은 변 하한에 맞추고 32의 배수로 반올림
pub(crate) fn det_size(w: u32, h: u32) -> (u32, u32) {
    let (w, h) = (w as f32, h as f32);
    let mut s = (DET_MAX_SIDE / w.max(h))
        .min((DET_MAX_PIXELS / (w * h)).sqrt())
        .min(1.0);
    if w.min(h) * s < DET_MIN_SIDE {
        s = DET_MIN_SIDE / w.min(h);
    }
    let round32 = |v: f32| (((v * s) / 32.0).round() as u32 * 32).max(32);
    (round32(w), round32(h))
}

fn det_input(img: &RgbImage, w: u32, h: u32) -> Vec<f32> {
    let small = resize(img, w, h, FilterType::Triangle);
    let plane = (w * h) as usize;
    let mut t = vec![0f32; 3 * plane];
    for (i, p) in small.pixels().enumerate() {
        let [r, g, b] = p.0;
        for (c, v) in [b, g, r].into_iter().enumerate() {
            t[c * plane + i] = (f32::from(v) / 255.0 - MEAN[c]) / STD[c];
        }
    }
    t
}

/// DB 확률 맵 → 글자 상자 (맵 좌표). 연결 요소의 바깥 사각형을 점수로 거르고 넓힌다(unclip).
// shortcut: 기울어진 글자도 축 정렬 사각형으로 잡는다(최소 넓이 회전 사각형 아님). 기울어진 글자가 많으면 회전 사각형으로
pub(crate) fn det_boxes(prob: &[f32], w: usize, h: usize) -> Vec<Rect> {
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    let mut out = Vec::new();
    for start in 0..w * h {
        if seen[start] || prob[start] <= DET_THRESH {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = ny * w + nx;
                    if !seen[j] && prob[j] > DET_THRESH {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        let (bw, bh) = ((x1 - x0) as f32, (y1 - y0) as f32);
        if bw.min(bh) < 3.0 {
            continue;
        }
        let mut sum = 0f32;
        for y in y0..=y1 {
            sum += prob[y * w + x0..=y * w + x1].iter().sum::<f32>();
        }
        if sum / (((x1 - x0 + 1) * (y1 - y0 + 1)) as f32) < BOX_THRESH {
            continue;
        }
        // DB는 줄어든 글자 중심을 예측한다: 넓이×비율/둘레만큼 바깥으로 넓힌다
        let d = bw * bh * UNCLIP_RATIO / (2.0 * (bw + bh));
        let r = Rect {
            x0: x0 as f32 - d,
            y0: y0 as f32 - d,
            x1: x1 as f32 + d,
            y1: y1 as f32 + d,
        };
        if r.w().min(r.h()) < 5.0 {
            continue;
        }
        out.push(r);
        if out.len() >= MAX_CANDIDATES {
            break;
        }
    }
    out
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
