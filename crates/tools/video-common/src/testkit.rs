//! 테스트용 영상 생성기 (feature "testkit")

use std::path::Path;

use windows::Win32::Media::MediaFoundation::*;
use windows::core::HSTRING;

use crate::mf::Session;

/// 시험 영상의 초당 프레임
pub const FPS: u32 = 15;

/// H.264 8Mbps mp4(`secs`초, 15fps). `fill(i, y)`가 i번째 프레임의 밝기(Y) 면을 채운다 (색 면은 회색).
/// 소리(48kHz 스테레오 440Hz 사인파) 선택
pub fn write_video(
    path: &Path,
    w: u32,
    h: u32,
    secs: u32,
    audio: bool,
    mut fill: impl FnMut(u32, &mut [u8]),
) {
    let _s = Session::new().unwrap();
    unsafe {
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 1).unwrap();
        let attrs = attrs.unwrap();
        attrs
            .SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)
            .unwrap();
        let writer =
            MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attrs).unwrap();
        let video_type = |subtype: &windows::core::GUID| {
            let t = MFCreateMediaType().unwrap();
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            t.SetGUID(&MF_MT_SUBTYPE, subtype).unwrap();
            t.SetUINT64(&MF_MT_FRAME_SIZE, ((w as u64) << 32) | h as u64)
                .unwrap();
            t.SetUINT64(&MF_MT_FRAME_RATE, ((FPS as u64) << 32) | 1)
                .unwrap();
            t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1 << 32) | 1)
                .unwrap();
            t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .unwrap();
            t
        };
        let out = video_type(&MFVideoFormat_H264);
        out.SetUINT32(&MF_MT_AVG_BITRATE, 8_000_000).unwrap();
        let v = writer.AddStream(&out).unwrap();
        writer
            .SetInputMediaType(v, &video_type(&MFVideoFormat_NV12), None)
            .unwrap();
        let audio_type = |subtype: &windows::core::GUID| {
            let t = MFCreateMediaType().unwrap();
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).unwrap();
            t.SetGUID(&MF_MT_SUBTYPE, subtype).unwrap();
            t.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).unwrap();
            t.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, 48_000)
                .unwrap();
            t.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).unwrap();
            t
        };
        let a = audio.then(|| {
            let aac = audio_type(&MFAudioFormat_AAC);
            aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 16_000)
                .unwrap();
            let a = writer.AddStream(&aac).unwrap();
            let pcm = audio_type(&MFAudioFormat_PCM);
            pcm.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 192_000)
                .unwrap();
            writer.SetInputMediaType(a, &pcm, None).unwrap();
            a
        });
        writer.BeginWriting().unwrap();

        let write = |stream: u32, bytes: &[u8], i: u32| {
            let buf = MFCreateMemoryBuffer(bytes.len() as u32).unwrap();
            let mut p = std::ptr::null_mut();
            buf.Lock(&mut p, None, None).unwrap();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            buf.Unlock().unwrap();
            buf.SetCurrentLength(bytes.len() as u32).unwrap();
            let s = MFCreateSample().unwrap();
            s.AddBuffer(&buf).unwrap();
            let dur = 10_000_000 / FPS as i64;
            s.SetSampleTime(i as i64 * dur).unwrap();
            s.SetSampleDuration(dur).unwrap();
            writer.WriteSample(stream, &s).unwrap();
        };
        let mut frame = vec![128u8; (w * h * 3 / 2) as usize];
        for i in 0..secs * FPS {
            fill(i, &mut frame[..(w * h) as usize]);
            write(v, &frame, i);
            if let Some(a) = a {
                let n = 48_000 / FPS;
                let pcm: Vec<u8> = (0..n)
                    .flat_map(|k| {
                        let t = (i * n + k) as f32 / 48_000.0;
                        let s = ((t * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
                        [s.to_le_bytes(), s.to_le_bytes()].concat()
                    })
                    .collect();
                write(a, &pcm, i);
            }
        }
        writer.Finalize().unwrap();
    }
}

/// 움직이는 줄무늬 + 잡음(인코더가 비트레이트를 다 쓰게)
pub fn make_video(path: &Path, w: u32, h: u32, secs: u32, audio: bool) {
    let mut seed = 1u32;
    write_video(path, w, h, secs, audio, |i, y| {
        for (k, p) in y.iter_mut().enumerate() {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let (x, row) = (k as u32 % w, k as u32 / w);
            *p = ((x + row + i * 8) % 200) as u8 + (seed >> 29) as u8 * 8;
        }
    });
}

/// 위 절반의 밝기가 시간에 따라 검정(Y 16)에서 흰색(Y 235)으로 오르고 아래 절반은 늘 검정.
/// 시각 → 밝기는 [`ramp_level`]
pub fn make_ramp(path: &Path, w: u32, h: u32, secs: u32) {
    let n = secs * FPS;
    write_video(path, w, h, secs, false, |i, y| {
        let (top, bottom) = y.split_at_mut((w * (h / 2)) as usize);
        top.fill(16 + (i * 219 / (n - 1).max(1)) as u8);
        bottom.fill(16);
    });
}

/// [`make_ramp`] 영상의 `secs`초에서 위 절반의 RGB 밝기 (0~255, 화면 범위)
pub fn ramp_level(total_secs: u32, secs: f64) -> f64 {
    let n = (total_secs * FPS) as f64;
    let i = (secs * FPS as f64).floor().min(n - 1.0);
    i * 255.0 / (n - 1.0)
}
