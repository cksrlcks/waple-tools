//! 영상을 RGBA 프레임으로 읽기 (썸네일·GIF).
//! 색 변환(→ RGB32)만 Source Reader에 맡기고 크기 조정은 직접 한다: 고급 영상 처리를 켜면 프레임 시각을 고르게 다시 매겨
//! 화면 녹화처럼 프레임 간격이 들쭉날쭉한 영상에서 시각이 틀어진다.

use std::path::Path;

use image::RgbaImage;
use image::imageops::{FilterType, crop_imm, resize};
use waple_tools_core::ToolError;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, Interface};

use crate::mf::{CODEC, Probe, Session, fail, open_reader, unpack, unreadable};

const FIRST_VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const READ: &str = "영상을 읽는 중 오류가 났습니다";

/// 디코딩된 한 프레임. 그림(RGBA)으로 바꾸는 건 [`FrameReader::image`]로 필요할 때만 한다
/// (탐색하면 키프레임부터 읽으므로 버릴 프레임이 많다)
pub struct Frame {
    /// 시작 시각 (100ns)
    pub time: i64,
    sample: IMFSample,
}

/// 영상 트랙을 원하는 크기의 RGBA로 차례대로 읽는다
pub struct FrameReader {
    // 필드는 선언 순서대로 해제된다: reader가 Session(MFShutdown)보다 먼저
    reader: IMFSourceReader,
    /// 결과 크기
    pub width: u32,
    pub height: u32,
    /// 원본 화면 크기 (디코더 버퍼가 16의 배수로 더 클 수 있어 잘라 낸다)
    shown: (u32, u32),
    /// 디코더 버퍼 크기와 한 줄 바이트 수 (음수면 아래에서 위로 저장된 그림)
    buffer: (u32, u32, i32),
    _s: Session,
}

impl FrameReader {
    /// `src`를 `width`×`height`로 읽는다 (원본 비율과 달라도 늘려 맞춘다)
    pub fn open(src: &Path, probe: &Probe, width: u32, height: u32) -> Result<Self, ToolError> {
        let s = Session::new()?;
        let reader = open_reader(src, false)?;
        unsafe {
            reader
                .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
                .map_err(|_| unreadable())?;
            reader
                .SetStreamSelection(FIRST_VIDEO, true)
                .map_err(|_| unreadable())?;
            let t = MFCreateMediaType().map_err(fail(CODEC))?;
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(fail(CODEC))?;
            t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
                .map_err(fail(CODEC))?;
            reader
                .SetCurrentMediaType(FIRST_VIDEO, None, &t)
                .map_err(fail(CODEC))?;
        }
        let mut r = Self {
            reader,
            width,
            height,
            shown: (probe.width, probe.height),
            buffer: (0, 0, 0),
            _s: s,
        };
        r.read_type().map_err(fail(CODEC))?;
        Ok(r)
    }

    fn read_type(&mut self) -> windows::core::Result<()> {
        let cur = unsafe { self.reader.GetCurrentMediaType(FIRST_VIDEO)? };
        let (w, h) = unpack(unsafe { cur.GetUINT64(&MF_MT_FRAME_SIZE)? });
        let stride = unsafe { cur.GetUINT32(&MF_MT_DEFAULT_STRIDE) }
            .map(|v| v as i32)
            .unwrap_or((w * 4) as i32);
        self.buffer = (w, h, stride);
        Ok(())
    }

    /// `time`(100ns) 쪽으로 옮긴다. 다음 프레임은 그 앞 키프레임부터 나올 수 있다
    pub fn seek(&self, time: u64) -> Result<(), ToolError> {
        let pos = PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_I8,
                    Anonymous: PROPVARIANT_0_0_0 { hVal: time as i64 },
                    ..Default::default()
                }),
            },
        };
        unsafe { self.reader.SetCurrentPosition(&GUID::zeroed(), &pos) }
            .map_err(fail("영상의 해당 시간으로 이동할 수 없습니다"))
    }

    /// 다음 프레임. 끝이면 `None`
    pub fn next_frame(&mut self) -> Result<Option<Frame>, ToolError> {
        loop {
            let (mut flags, mut time) = (0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            unsafe {
                self.reader
                    .ReadSample(
                        FIRST_VIDEO,
                        0,
                        None,
                        Some(&mut flags),
                        Some(&mut time),
                        Some(&mut sample),
                    )
                    .map_err(fail(READ))?;
            }
            if flags & MF_SOURCE_READERF_ERROR.0 as u32 != 0 {
                return Err(ToolError::Failed(READ.into()));
            }
            if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
                self.read_type().map_err(fail(READ))?;
            }
            if let Some(sample) = sample {
                return Ok(Some(Frame { time, sample }));
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Ok(None);
            }
        }
    }

    /// `width`×`height` RGBA (불투명)
    pub fn image(&self, f: &Frame) -> Result<RgbaImage, ToolError> {
        let img = unsafe { self.copy(&f.sample) }.map_err(fail(READ))?;
        Ok(self.fit(img))
    }

    /// 디코더 버퍼 → 원본 화면 크기로 자르고 결과 크기로 줄인다
    fn fit(&self, img: RgbaImage) -> RgbaImage {
        let (sw, sh) = (
            self.shown.0.min(img.width()),
            self.shown.1.min(img.height()),
        );
        let img = if (sw, sh) == img.dimensions() {
            img
        } else {
            crop_imm(&img, 0, 0, sw, sh).to_image()
        };
        if img.dimensions() == (self.width, self.height) {
            img
        } else {
            resize(&img, self.width, self.height, FilterType::Triangle)
        }
    }

    /// BGRX → RGBA. 2D 버퍼면 그 줄 간격(음수 = 뒤집힘)을 따른다
    unsafe fn copy(&self, sample: &IMFSample) -> windows::core::Result<RgbaImage> {
        let (bw, bh, stride) = self.buffer;
        let (w, h) = (bw as usize, bh as usize);
        let mut out = vec![0u8; w * h * 4];
        let mut fill = |top: *const u8, pitch: isize| {
            for (y, row) in out.chunks_exact_mut(w * 4).enumerate() {
                let src =
                    unsafe { std::slice::from_raw_parts(top.offset(y as isize * pitch), w * 4) };
                for (d, s) in row
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(src.as_chunks::<4>().0)
                {
                    *d = [s[2], s[1], s[0], 255];
                }
            }
        };
        let short = || windows::core::Error::from(windows::core::HRESULT(0x8000_FFFF_u32 as i32));
        unsafe {
            let buf = sample.ConvertToContiguousBuffer()?;
            if let Ok(b2) = buf.cast::<IMF2DBuffer>() {
                let (mut top, mut pitch) = (std::ptr::null_mut(), 0i32);
                b2.Lock2D(&mut top, &mut pitch)?;
                fill(top, pitch as isize);
                b2.Unlock2D()?;
            } else {
                let (mut p, mut len) = (std::ptr::null_mut(), 0u32);
                buf.Lock(&mut p, None, Some(&mut len))?;
                let pitch = stride as isize;
                if (len as usize) < pitch.unsigned_abs() * h {
                    buf.Unlock()?;
                    return Err(short());
                }
                let top = if pitch < 0 {
                    p.add(pitch.unsigned_abs() * (h - 1))
                } else {
                    p
                };
                fill(top, pitch);
                buf.Unlock()?;
            }
        }
        RgbaImage::from_raw(bw, bh, out).ok_or_else(short)
    }
}
