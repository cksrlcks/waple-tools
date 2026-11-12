//! RGBA 프레임 → H.264 mp4 (GIF → 영상).
//! 색 변환(RGBA → NV12)은 직접 한다: RGB32를 넘기면 Sink Writer가 끼우는 영상 처리기가 초당 프레임을 고정해 프레임마다 다른 지연이 사라진다.

use std::path::Path;

use waple_tools_core::ToolError;
use windows::Win32::Media::MediaFoundation::*;

use crate::mf::{SAVE, Session, cbr, fail, open_writer, video_type};

/// H.264 소프트웨어 인코더가 받는 크기 범위 (가로·세로 각각). 최소는 Windows 11에서 잰 값(32 실패, 34 성공)에 여유를 둔 것, 최대는 레벨 5.2
pub const MIN_ENCODE_SIDE: u32 = 48;
pub const MAX_ENCODE_SIDE: u32 = 4096;

pub struct Mp4Writer {
    // 필드는 선언 순서대로 해제된다: writer가 Session보다 먼저
    writer: IMFSinkWriter,
    stream: u32,
    width: u32,
    height: u32,
    bt709: bool,
    _s: Session,
}

/// RGBA → NV12 (TV 범위 16~235). `bt709`면 HD 색 공식, 아니면 SD(BT.601)
fn nv12(rgba: &[u8], w: usize, h: usize, bt709: bool, out: &mut [u8]) {
    // (Y, Cb, Cr) 행 = R·G·B 계수 (÷255 전)
    let m: [[f32; 3]; 3] = if bt709 {
        [
            [46.559, 156.629, 15.812],
            [-25.664, -86.336, 112.0],
            [112.0, -101.730, -10.270],
        ]
    } else {
        [
            [65.481, 128.553, 24.966],
            [-37.797, -74.203, 112.0],
            [112.0, -93.786, -18.214],
        ]
    };
    let dot = |r: &[f32; 3], p: &[u8]| {
        (r[0] * f32::from(p[0]) + r[1] * f32::from(p[1]) + r[2] * f32::from(p[2])) / 255.0
    };
    let (y_plane, uv) = out.split_at_mut(w * h);
    for (d, p) in y_plane.iter_mut().zip(rgba.as_chunks::<4>().0) {
        *d = (16.0 + dot(&m[0], p)).round() as u8;
    }
    // 색 면: 2×2 평균
    for by in 0..h / 2 {
        for bx in 0..w / 2 {
            let mut avg = [0f32; 3];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = ((by * 2 + dy) * w + bx * 2 + dx) * 4;
                for (a, c) in avg.iter_mut().zip(&rgba[i..i + 3]) {
                    *a += f32::from(*c) / 4.0;
                }
            }
            let p = avg.map(|v| v.round() as u8);
            uv[by * w + bx * 2] = (128.0 + dot(&m[1], &p)).round() as u8;
            uv[by * w + bx * 2 + 1] = (128.0 + dot(&m[2], &p)).round() as u8;
        }
    }
}

impl Mp4Writer {
    /// `width`·`height`는 짝수, [`MIN_ENCODE_SIDE`]~[`MAX_ENCODE_SIDE`]. `fps`는 인코더의 비트 배분 기준 (실제 시각은 프레임마다 준다).
    /// 그래픽카드 인코더는 작은 크기에서 실패해서(Intel Quick Sync, 64×48) Windows 소프트웨어 인코더만 쓴다
    pub fn create(
        dst: &Path,
        width: u32,
        height: u32,
        fps: (u32, u32),
        bps: u32,
    ) -> Result<Self, ToolError> {
        let ok = |v: u32| v.is_multiple_of(2) && (MIN_ENCODE_SIDE..=MAX_ENCODE_SIDE).contains(&v);
        if !ok(width) || !ok(height) {
            return Err(ToolError::Failed(format!(
                "영상으로 만들 수 없는 크기입니다 ({width}×{height}). 가로·세로는 {MIN_ENCODE_SIDE}~{MAX_ENCODE_SIDE}px 짝수여야 합니다."
            )));
        }
        let s = Session::new()?;
        let writer = open_writer(dst, &MFTranscodeContainerType_MPEG4, false)?;
        unsafe {
            let out = video_type(&MFVideoFormat_H264, width, height, fps, SAVE)?;
            out.SetUINT32(&MF_MT_AVG_BITRATE, bps).map_err(fail(SAVE))?;
            out.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)
                .map_err(fail(SAVE))?;
            // 재생기는 보통 HD 크기부터 BT.709로 본다
            let bt709 = width >= 1280 || height >= 720;
            let input = video_type(&MFVideoFormat_NV12, width, height, fps, SAVE)?;
            for t in [&out, &input] {
                t.SetUINT32(
                    &MF_MT_YUV_MATRIX,
                    if bt709 {
                        MFVideoTransferMatrix_BT709.0
                    } else {
                        MFVideoTransferMatrix_BT601.0
                    } as u32,
                )
                .map_err(fail(SAVE))?;
                t.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)
                    .map_err(fail(SAVE))?;
            }
            let stream = writer.AddStream(&out).map_err(fail(SAVE))?;
            let params = cbr(bps)?;
            // B 프레임을 쓰면 mp4 저장기가 프레임마다 다른 시각을 순서 바뀐 채로 적는다 (ctts)
            params
                .SetUINT32(&CODECAPI_AVEncMPVDefaultBPictureCount, 0)
                .map_err(fail(SAVE))?;
            writer
                .SetInputMediaType(stream, &input, &params)
                .map_err(fail(SAVE))?;
            writer.BeginWriting().map_err(fail(SAVE))?;
            Ok(Self {
                writer,
                stream,
                width,
                height,
                bt709,
                _s: s,
            })
        }
    }

    /// RGBA 한 프레임을 `time`부터 `duration`(100ns) 동안 보여 준다. 알파는 무시한다
    pub fn write(&self, rgba: &[u8], time: i64, duration: i64) -> Result<(), ToolError> {
        let (w, h) = (self.width as usize, self.height as usize);
        if rgba.len() != w * h * 4 {
            return Err(ToolError::Failed(SAVE.into()));
        }
        let len = w * h * 3 / 2;
        unsafe {
            let buf = MFCreateMemoryBuffer(len as u32).map_err(fail(SAVE))?;
            let mut p = std::ptr::null_mut();
            buf.Lock(&mut p, None, None).map_err(fail(SAVE))?;
            nv12(
                rgba,
                w,
                h,
                self.bt709,
                std::slice::from_raw_parts_mut(p, len),
            );
            buf.Unlock().map_err(fail(SAVE))?;
            buf.SetCurrentLength(len as u32).map_err(fail(SAVE))?;
            let sample = MFCreateSample().map_err(fail(SAVE))?;
            sample.AddBuffer(&buf).map_err(fail(SAVE))?;
            sample.SetSampleTime(time).map_err(fail(SAVE))?;
            sample.SetSampleDuration(duration).map_err(fail(SAVE))?;
            self.writer
                .WriteSample(self.stream, &sample)
                .map_err(fail(SAVE))
        }
    }

    pub fn finish(self) -> Result<(), ToolError> {
        unsafe { self.writer.Finalize() }.map_err(fail(SAVE))
    }
}
