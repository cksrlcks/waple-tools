//! PaddleOCR DB 글자 영역 검출 (PP-OCRv5 server·mobile det 공통): 전처리 → 추론 → 상자.
//! 두 모델의 전처리(ImageNet 정규화, BGR)와 후처리(DBPostProcess) 기본값이 같다.

use image::RgbImage;
use image::imageops::{FilterType, resize};
use waple_tools_core::ToolError;

use crate::Model;

/// det 입력 긴 변 상한·짧은 변 하한 (PaddleOCR det 기본값)
const DET_MAX_SIDE: f32 = 4000.0;
const DET_MIN_SIDE: f32 = 64.0;
/// det 입력 화소 수 상한 (CPU 시간). 긴 변 기준으로 줄이면 세로로 긴 공지의 글자가 너무 작아져 넓이로 줄인다
const DET_MAX_PIXELS: f32 = 2_500_000.0;
/// DB 후처리 (PaddleOCR PP-OCRv5 det 기본값)
const DET_THRESH: f32 = 0.3;
const BOX_THRESH: f32 = 0.6;
const UNCLIP_RATIO: f32 = 1.5;
const MAX_CANDIDATES: usize = 1000;

/// 정규화 (ImageNet, BGR 순서로 넣는다: PaddleOCR가 OpenCV BGR 그대로 정규화)
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// 글자 상자 (이미지 좌표, 이미지 안으로 자른 값)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// 이미지에서 글자 줄 상자를 찾는다
pub fn detect(model: &Model, img: &RgbImage) -> Result<Vec<DetBox>, ToolError> {
    model.with_session(|r| {
        let (rw, rh) = det_size(img.width(), img.height());
        let input = det_input(img, rw, rh);
        let (shape, prob) = r.infer_shaped(vec![1, 3, rh as usize, rw as usize], input)?;
        let (h, w) = match shape[..] {
            [1, 1, h, w] if h * w == prob.len() => (h, w),
            _ => {
                return Err(ToolError::Failed(
                    "OCR 모델의 출력 모양이 예상과 다릅니다.".into(),
                ));
            }
        };
        let (iw, ih) = (img.width() as f32, img.height() as f32);
        let (sx, sy) = (iw / w as f32, ih / h as f32);
        Ok(det_boxes(&prob, w, h)
            .into_iter()
            .map(|b| DetBox {
                x0: (b.x0 * sx).clamp(0.0, iw),
                y0: (b.y0 * sy).clamp(0.0, ih),
                x1: (b.x1 * sx).clamp(0.0, iw),
                y1: (b.y1 * sy).clamp(0.0, ih),
            })
            .collect())
    })
}

/// det 입력 크기: 화소 수·긴 변 상한, 짧은 변 하한에 맞추고 32의 배수로 반올림
pub fn det_size(w: u32, h: u32) -> (u32, u32) {
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
pub fn det_boxes(prob: &[f32], w: usize, h: usize) -> Vec<DetBox> {
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
        let r = DetBox {
            x0: x0 as f32 - d,
            y0: y0 as f32 - d,
            x1: x1 as f32 + d,
            y1: y1 as f32 + d,
        };
        if (r.x1 - r.x0).min(r.y1 - r.y0) < 5.0 {
            continue;
        }
        out.push(r);
        if out.len() >= MAX_CANDIDATES {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn det_input_size_keeps_tall_posters_readable() {
        // 작은 배너는 그대로(32 배수)
        assert_eq!(det_size(720, 520), (736, 512));
        // 세로로 긴 공지: 긴 변이 아니라 넓이로 줄여 폭이 충분히 남는다
        let (w, h) = det_size(1080, 5000);
        assert!(w >= 700 && h <= 4000, "{w}×{h}");
        assert!(w as f32 * h as f32 <= 2_600_000.0);
        // 아주 작은 이미지는 짧은 변 64까지 키운다
        assert_eq!(det_size(400, 20).1, 64);
    }
}
