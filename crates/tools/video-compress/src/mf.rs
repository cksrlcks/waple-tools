//! Windows Media Foundation으로 영상 읽기·쓰기. 디코딩은 Source Reader, 인코딩(H.264·AAC)과 mp4 저장은 Sink Writer가 한다.

use std::path::Path;

use waple_tools_core::{JobContext, ToolError};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::{GUID, HSTRING};

/// 원본 영상 정보
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub width: u32,
    pub height: u32,
    /// 초당 프레임 (분자, 분모)
    pub fps: (u32, u32),
    /// 100ns 단위
    pub duration: u64,
    pub has_audio: bool,
}

/// 결과 영상 설정
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub width: u32,
    pub height: u32,
    pub video_bps: u32,
    pub audio: bool,
}

/// AAC 비트레이트 (128kbps)
pub const AUDIO_BPS: u32 = 128_000;

const FIRST_VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const FIRST_AUDIO: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;

/// COM·Media Foundation 시작/종료 (스레드마다)
pub(crate) struct Session {
    com: bool,
}

impl Session {
    pub(crate) fn new() -> Result<Self, ToolError> {
        // 이미 다른 방식(STA)으로 초기화된 스레드면 그대로 쓴다 (MF는 둘 다 된다)
        let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        if let Err(e) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
            if com {
                unsafe { CoUninitialize() };
            }
            return Err(ToolError::Failed(format!(
                "Windows 미디어 기능을 시작할 수 없습니다. Windows N 에디션이면 '미디어 기능 팩'을 설치해 주세요. ({e})"
            )));
        }
        Ok(Self { com })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            if self.com {
                CoUninitialize();
            }
        }
    }
}

fn fail(msg: &str) -> impl Fn(windows::core::Error) -> ToolError + '_ {
    move |e| ToolError::Failed(format!("{msg} ({})", e.message()))
}

fn unreadable() -> ToolError {
    ToolError::Failed("영상을 읽을 수 없습니다 (손상되었거나 영상 파일이 아닙니다).".into())
}

fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

fn unpack(v: u64) -> (u32, u32) {
    ((v >> 32) as u32, v as u32)
}

fn open_reader(src: &Path) -> Result<IMFSourceReader, ToolError> {
    unsafe {
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 1).map_err(fail("영상을 열 수 없습니다"))?;
        let attrs = attrs.ok_or_else(unreadable)?;
        // 크기 줄이기·색 변환(NV12)을 Source Reader가 한다
        attrs
            .SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)
            .map_err(fail("영상을 열 수 없습니다"))?;
        MFCreateSourceReaderFromURL(&HSTRING::from(src.as_os_str()), &attrs)
            .map_err(|_| unreadable())
    }
}

fn read_probe(reader: &IMFSourceReader) -> Result<Probe, ToolError> {
    unsafe {
        let video = reader
            .GetNativeMediaType(FIRST_VIDEO, 0)
            .map_err(|_| ToolError::Failed("영상 트랙이 없는 파일입니다.".into()))?;
        let (width, height) = unpack(
            video
                .GetUINT64(&MF_MT_FRAME_SIZE)
                .map_err(|_| unreadable())?,
        );
        let fps = video
            .GetUINT64(&MF_MT_FRAME_RATE)
            .map(unpack)
            .ok()
            .filter(|&(n, d)| n > 0 && d > 0)
            .unwrap_or((30, 1));
        let pv = reader
            .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
            .map_err(|_| unreadable())?;
        let duration = pv.Anonymous.Anonymous.Anonymous.uhVal;
        let has_audio = reader.GetNativeMediaType(FIRST_AUDIO, 0).is_ok();
        if width == 0 || height == 0 || duration == 0 {
            return Err(unreadable());
        }
        Ok(Probe {
            width,
            height,
            fps,
            duration,
            has_audio,
        })
    }
}

pub fn probe(src: &Path) -> Result<Probe, ToolError> {
    let _s = Session::new()?;
    read_probe(&open_reader(src)?)
}

/// `src`를 `plan`대로 H.264·AAC mp4 `dst`로 다시 인코딩한다. 진행률은 영상 시간 기준.
pub fn transcode(
    src: &Path,
    dst: &Path,
    probe: &Probe,
    plan: &Plan,
    ctx: &JobContext,
) -> Result<(), ToolError> {
    let _s = Session::new()?;
    let reader = open_reader(src)?;
    unsafe {
        let codec = "이 영상의 코덱을 읽을 수 없습니다. HEVC(H.265) 영상은 Microsoft Store의 'HEVC 비디오 확장'이 필요합니다.";
        reader
            .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
            .map_err(|_| unreadable())?;
        reader
            .SetStreamSelection(FIRST_VIDEO, true)
            .map_err(|_| unreadable())?;
        let frame = pack(plan.width, plan.height);
        let fps = pack(probe.fps.0, probe.fps.1);
        let raw = MFCreateMediaType().map_err(fail(codec))?;
        raw.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(fail(codec))?;
        raw.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)
            .map_err(fail(codec))?;
        raw.SetUINT64(&MF_MT_FRAME_SIZE, frame)
            .map_err(fail(codec))?;
        raw.SetUINT64(&MF_MT_FRAME_RATE, fps).map_err(fail(codec))?;
        raw.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))
            .map_err(fail(codec))?;
        raw.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(fail(codec))?;
        reader
            .SetCurrentMediaType(FIRST_VIDEO, None, &raw)
            .map_err(fail(codec))?;
        let raw = reader
            .GetCurrentMediaType(FIRST_VIDEO)
            .map_err(fail(codec))?;

        let pcm = if plan.audio {
            reader
                .SetStreamSelection(FIRST_AUDIO, true)
                .map_err(|_| unreadable())?;
            let t = MFCreateMediaType().map_err(fail(codec))?;
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
                .map_err(fail(codec))?;
            t.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)
                .map_err(fail(codec))?;
            t.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)
                .map_err(fail(codec))?;
            reader
                .SetCurrentMediaType(FIRST_AUDIO, None, &t)
                .map_err(fail(
                    "이 영상의 오디오를 읽을 수 없습니다. '소리 유지'를 끄고 다시 해 보세요.",
                ))?;
            Some(
                reader
                    .GetCurrentMediaType(FIRST_AUDIO)
                    .map_err(fail(codec))?,
            )
        } else {
            None
        };

        let save = "결과 영상을 만들 수 없습니다";
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 2).map_err(fail(save))?;
        let attrs = attrs.ok_or_else(|| ToolError::Failed(save.into()))?;
        // 그래픽카드 인코더가 있으면 쓴다 (없으면 Windows 소프트웨어 인코더)
        attrs
            .SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)
            .map_err(fail(save))?;
        attrs
            .SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)
            .map_err(fail(save))?;
        let writer = MFCreateSinkWriterFromURL(&HSTRING::from(dst.as_os_str()), None, &attrs)
            .map_err(fail(save))?;

        let h264 = MFCreateMediaType().map_err(fail(save))?;
        h264.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(fail(save))?;
        h264.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
            .map_err(fail(save))?;
        h264.SetUINT32(&MF_MT_AVG_BITRATE, plan.video_bps)
            .map_err(fail(save))?;
        h264.SetUINT64(&MF_MT_FRAME_SIZE, frame)
            .map_err(fail(save))?;
        h264.SetUINT64(&MF_MT_FRAME_RATE, fps).map_err(fail(save))?;
        h264.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))
            .map_err(fail(save))?;
        h264.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(fail(save))?;
        h264.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)
            .map_err(fail(save))?;
        // ponytail: 휴대폰 영상의 회전 정보를 그대로 넘긴다. mp4 저장기가 무시하는 Windows 버전이면 눕혀 보일 수 있다
        if let Ok(rot) = reader
            .GetNativeMediaType(FIRST_VIDEO, 0)
            .and_then(|t| t.GetUINT32(&MF_MT_VIDEO_ROTATION))
        {
            let _ = h264.SetUINT32(&MF_MT_VIDEO_ROTATION, rot);
        }
        let vout = writer.AddStream(&h264).map_err(fail(save))?;
        // 고정 비트레이트: 목표 용량을 맞추기 쉽다
        let mut params = None;
        MFCreateAttributes(&mut params, 2).map_err(fail(save))?;
        let params = params.ok_or_else(|| ToolError::Failed(save.into()))?;
        params
            .SetUINT32(
                &CODECAPI_AVEncCommonRateControlMode,
                eAVEncCommonRateControlMode_CBR.0 as u32,
            )
            .map_err(fail(save))?;
        params
            .SetUINT32(&CODECAPI_AVEncCommonMeanBitRate, plan.video_bps)
            .map_err(fail(save))?;
        writer
            .SetInputMediaType(vout, &raw, &params)
            .map_err(fail(save))?;

        let aout = match &pcm {
            Some(pcm) => {
                let rate = pcm.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).unwrap_or(0);
                let ch = pcm.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).unwrap_or(0);
                let aac_fail = |_| {
                    ToolError::Failed(format!(
                        "오디오({rate}Hz, {ch}채널)를 AAC로 바꿀 수 없습니다. '소리 유지'를 끄고 다시 해 보세요."
                    ))
                };
                let aac = MFCreateMediaType().map_err(fail(save))?;
                aac.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
                    .map_err(fail(save))?;
                aac.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC)
                    .map_err(fail(save))?;
                aac.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)
                    .map_err(fail(save))?;
                aac.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate)
                    .map_err(fail(save))?;
                aac.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, ch)
                    .map_err(fail(save))?;
                aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, AUDIO_BPS / 8)
                    .map_err(fail(save))?;
                let i = writer.AddStream(&aac).map_err(aac_fail)?;
                writer.SetInputMediaType(i, pcm, None).map_err(aac_fail)?;
                Some(i)
            }
            None => None,
        };

        writer.BeginWriting().map_err(fail(save))?;

        // 실제 스트림 번호 → 출력 스트림
        let mut targets: Vec<(u32, u32)> = vec![];
        let mut i = 0;
        while let Ok(selected) = reader.GetStreamSelection(i) {
            if selected.as_bool() {
                let major = reader
                    .GetNativeMediaType(i, 0)
                    .and_then(|t| t.GetGUID(&MF_MT_MAJOR_TYPE))
                    .unwrap_or(GUID::zeroed());
                if major == MFMediaType_Video {
                    targets.push((i, vout));
                } else if let (true, Some(a)) = (major == MFMediaType_Audio, aout) {
                    targets.push((i, a));
                }
            }
            i += 1;
        }

        let mut open = targets.len();
        let read = "영상을 읽는 중 오류가 났습니다";
        while open > 0 {
            ctx.check_cancel()?;
            let (mut idx, mut flags, mut ts) = (0u32, 0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            reader
                .ReadSample(
                    MF_SOURCE_READER_ANY_STREAM.0 as u32,
                    0,
                    Some(&mut idx),
                    Some(&mut flags),
                    Some(&mut ts),
                    Some(&mut sample),
                )
                .map_err(fail(read))?;
            if flags & MF_SOURCE_READERF_ERROR.0 as u32 != 0 {
                return Err(ToolError::Failed(read.into()));
            }
            let Some(&(_, out)) = targets.iter().find(|(s, _)| *s == idx) else {
                continue;
            };
            if let Some(sample) = &sample {
                writer.WriteSample(out, sample).map_err(fail(save))?;
                if out == vout {
                    ctx.progress(ts as f32 / probe.duration as f32);
                }
            } else if flags & MF_SOURCE_READERF_STREAMTICK.0 as u32 != 0 {
                writer.SendStreamTick(out, ts).map_err(fail(save))?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                open -= 1;
            }
        }
        writer.Finalize().map_err(fail(save))?;
    }
    Ok(())
}
