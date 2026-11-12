//! Windows Media Foundation으로 영상 읽기·쓰기. 디코딩은 Source Reader, 인코딩과 파일 저장은 Sink Writer가 한다.

use std::path::Path;

use waple_tools_core::{JobContext, ToolError};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::{GUID, HSTRING, Interface};

/// 원본 영상 정보
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    /// 저장된 프레임 크기 (회전 전, 코덱이 덧댄 여백은 뺀 화면 크기)
    pub width: u32,
    pub height: u32,
    /// 초당 프레임 (분자, 분모). 머리 정보가 프레임 시각과 크게 다르면 시각에서 잰 값
    pub fps: (u32, u32),
    /// 100ns 단위. 0이면 모름 (MediaRecorder로 녹화한 webm 등)
    pub duration: u64,
    pub has_audio: bool,
    /// 보여 줄 때 시계 방향으로 돌릴 각도 (0/90/180/270, 휴대폰 세로 영상은 보통 90)
    pub rotation: u32,
}

impl Probe {
    /// 회전을 반영한 화면 크기
    pub fn shown_size(&self) -> (u32, u32) {
        if self.rotation % 180 == 90 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        }
    }

    /// 길이(초). 모르면 `None`
    pub fn secs(&self) -> Option<f64> {
        (self.duration > 0).then(|| self.duration as f64 / 1e7)
    }
}

/// 결과 파일 형식. 모두 Windows에 기본으로 든 인코더다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// H.264 + AAC
    Mp4,
    /// WMV9 + WMA
    Wmv,
    /// 소리만 AAC
    M4a,
    /// 소리만 MP3
    Mp3,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Wmv => "wmv",
            Self::M4a => "m4a",
            Self::Mp3 => "mp3",
        }
    }

    pub fn has_video(self) -> bool {
        matches!(self, Self::Mp4 | Self::Wmv)
    }

    fn container(self) -> GUID {
        match self {
            Self::Mp4 | Self::M4a => MFTranscodeContainerType_MPEG4,
            Self::Wmv => MFTranscodeContainerType_ASF,
            Self::Mp3 => MFTranscodeContainerType_MP3,
        }
    }

    /// (오디오 코덱, 오류 문구용 이름)
    fn audio(self) -> (GUID, &'static str) {
        match self {
            Self::Mp4 | Self::M4a => (MFAudioFormat_AAC, "AAC"),
            Self::Wmv => (MFAudioFormat_WMAudioV8, "WMA"),
            Self::Mp3 => (MFAudioFormat_MP3, "MP3"),
        }
    }
}

/// 결과 설정. 소리만 뽑는 형식이면 크기·영상 비트레이트는 쓰지 않는다
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub video_bps: u32,
    pub audio: bool,
}

/// 오디오 비트레이트 (128kbps)
pub const AUDIO_BPS: u32 = 128_000;

const FIRST_VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const FIRST_AUDIO: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;

/// COM·Media Foundation 시작/종료 (스레드마다)
pub struct Session {
    com: bool,
}

impl Session {
    pub fn new() -> Result<Self, ToolError> {
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

pub(crate) fn fail(msg: &str) -> impl Fn(windows::core::Error) -> ToolError + '_ {
    move |e| ToolError::Failed(format!("{msg} ({})", e.message()))
}

pub(crate) fn unreadable() -> ToolError {
    ToolError::Failed("영상을 읽을 수 없습니다 (손상되었거나 영상 파일이 아닙니다).".into())
}

pub(crate) const CODEC: &str = "이 영상의 코덱을 읽을 수 없습니다. Windows에 해당 코덱이 없을 수 있습니다 (HEVC·VP9·AV1 영상은 Microsoft Store의 'HEVC 비디오 확장'·'VP9 비디오 확장'·'AV1 비디오 확장')";
pub(crate) const READ: &str = "영상을 읽는 중 오류가 났습니다. 손상되었거나 Windows에 해당 코덱이 없을 수 있습니다 (HEVC·VP9·AV1 영상은 Microsoft Store의 'HEVC 비디오 확장'·'VP9 비디오 확장'·'AV1 비디오 확장')";
pub(crate) const SAVE: &str = "결과 파일을 만들 수 없습니다";

pub(crate) fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

pub(crate) fn unpack(v: u64) -> (u32, u32) {
    ((v >> 32) as u32, v as u32)
}

/// `advanced`면 크기 줄이기·색 변환(NV12)까지, 아니면 색 변환(RGB32)만 Source Reader가 한다
pub(crate) fn open_reader(src: &Path, advanced: bool) -> Result<IMFSourceReader, ToolError> {
    unsafe {
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 1).map_err(fail("영상을 열 수 없습니다"))?;
        let attrs = attrs.ok_or_else(unreadable)?;
        let key = if advanced {
            MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING
        } else {
            MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING
        };
        attrs
            .SetUINT32(&key, 1)
            .map_err(fail("영상을 열 수 없습니다"))?;
        MFCreateSourceReaderFromURL(&HSTRING::from(src.as_os_str()), &attrs)
            .map_err(|_| unreadable())
    }
}

/// 비압축·압축 공용 영상 형식 (정사각 화소, 순차 주사)
pub(crate) fn video_type(
    subtype: &GUID,
    w: u32,
    h: u32,
    fps: (u32, u32),
    msg: &str,
) -> Result<IMFMediaType, ToolError> {
    unsafe {
        let t = MFCreateMediaType().map_err(fail(msg))?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(fail(msg))?;
        t.SetGUID(&MF_MT_SUBTYPE, subtype).map_err(fail(msg))?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, pack(w, h))
            .map_err(fail(msg))?;
        t.SetUINT64(&MF_MT_FRAME_RATE, pack(fps.0, fps.1))
            .map_err(fail(msg))?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))
            .map_err(fail(msg))?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(fail(msg))?;
        Ok(t)
    }
}

/// 고정 비트레이트 인코더 설정: 목표 용량을 맞추기 쉽다
pub(crate) fn cbr(bps: u32) -> Result<IMFAttributes, ToolError> {
    unsafe {
        let mut params = None;
        MFCreateAttributes(&mut params, 2).map_err(fail(SAVE))?;
        let params = params.ok_or_else(|| ToolError::Failed(SAVE.into()))?;
        params
            .SetUINT32(
                &CODECAPI_AVEncCommonRateControlMode,
                eAVEncCommonRateControlMode_CBR.0 as u32,
            )
            .map_err(fail(SAVE))?;
        params
            .SetUINT32(&CODECAPI_AVEncCommonMeanBitRate, bps)
            .map_err(fail(SAVE))?;
        Ok(params)
    }
}

/// `dst`에 `container` 형식으로 쓰는 Sink Writer. `hardware`면 그래픽카드 인코더가 있을 때 쓴다
pub(crate) fn open_writer(
    dst: &Path,
    container: &GUID,
    hardware: bool,
) -> Result<IMFSinkWriter, ToolError> {
    unsafe {
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 2).map_err(fail(SAVE))?;
        let attrs = attrs.ok_or_else(|| ToolError::Failed(SAVE.into()))?;
        attrs
            .SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, hardware.into())
            .map_err(fail(SAVE))?;
        attrs
            .SetGUID(&MF_TRANSCODE_CONTAINERTYPE, container)
            .map_err(fail(SAVE))?;
        MFCreateSinkWriterFromURL(&HSTRING::from(dst.as_os_str()), None, &attrs).map_err(fail(SAVE))
    }
}

/// 프레임 시각들(정렬 안 됨, 100ns)에서 잰 초당 프레임
pub(crate) fn measured_fps(times: &[i64]) -> Option<f64> {
    let (lo, hi) = (times.iter().min()?, times.iter().max()?);
    (times.len() >= 2 && hi > lo).then(|| (times.len() - 1) as f64 * 1e7 / (hi - lo) as f64)
}

/// 머리 정보의 초당 프레임이 실제 프레임 시각과 20% 넘게 다르면 잰 값을 쓴다 (MKV·WebM은 절반으로 보고하기도 한다)
pub(crate) fn pick_fps(header: Option<(u32, u32)>, measured: Option<f64>) -> (u32, u32) {
    let header = header.filter(|&(n, d)| n > 0 && d > 0);
    match (header, measured) {
        (Some((n, d)), Some(m)) if ((n as f64 / d as f64) / m - 1.0).abs() <= 0.2 => (n, d),
        (_, Some(m)) if m.is_finite() && m > 0.0 => (((m * 1000.0).round() as u32).max(1), 1000),
        (Some(h), _) => h,
        _ => (30, 1),
    }
}

fn read_probe(reader: &IMFSourceReader) -> Result<Probe, ToolError> {
    unsafe {
        let video = reader
            .GetNativeMediaType(FIRST_VIDEO, 0)
            .map_err(|_| ToolError::Failed("영상 트랙이 없는 파일입니다.".into()))?;
        let (mut width, mut height) = unpack(
            video
                .GetUINT64(&MF_MT_FRAME_SIZE)
                .map_err(|_| unreadable())?,
        );
        // 코덱이 8·16의 배수로 덧댄 크기(320×184)보다 실제 화면 크기(320×180)
        let mut area = [0u8; 16];
        if video
            .GetBlob(&MF_MT_MINIMUM_DISPLAY_APERTURE, &mut area, None)
            .is_ok()
        {
            let (cx, cy) = (
                i32::from_le_bytes(area[8..12].try_into().unwrap_or_default()),
                i32::from_le_bytes(area[12..16].try_into().unwrap_or_default()),
            );
            if cx > 0 && cy > 0 && cx as u32 <= width && cy as u32 <= height {
                (width, height) = (cx as u32, cy as u32);
            }
        }
        let rotation = video.GetUINT32(&MF_MT_VIDEO_ROTATION).unwrap_or(0) % 360;
        let header = video.GetUINT64(&MF_MT_FRAME_RATE).map(unpack).ok();
        let duration = reader
            .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
            .map(|pv| pv.Anonymous.Anonymous.Anonymous.uhVal)
            .unwrap_or(0);
        let has_audio = reader.GetNativeMediaType(FIRST_AUDIO, 0).is_ok();
        if width == 0 || height == 0 {
            return Err(unreadable());
        }
        // 압축된 채로 앞 프레임 몇 개의 시각만 본다 (디코딩 안 함)
        let _ = reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false);
        let _ = reader.SetStreamSelection(FIRST_VIDEO, true);
        let mut times = vec![];
        while times.len() < 30 {
            let (mut flags, mut ts) = (0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            if reader
                .ReadSample(
                    FIRST_VIDEO,
                    0,
                    None,
                    Some(&mut flags),
                    Some(&mut ts),
                    Some(&mut sample),
                )
                .is_err()
                || flags & (MF_SOURCE_READERF_ERROR.0 | MF_SOURCE_READERF_ENDOFSTREAM.0) as u32 != 0
            {
                break;
            }
            if sample.is_some() {
                times.push(ts);
            }
        }
        Ok(Probe {
            width,
            height,
            fps: pick_fps(header, measured_fps(&times)),
            duration,
            has_audio,
            rotation,
        })
    }
}

pub fn probe(src: &Path) -> Result<Probe, ToolError> {
    let _s = Session::new()?;
    read_probe(&open_reader(src, true)?)
}

/// 소리 인코더가 받는 (표본 빈도, 채널). AAC는 44.1·48kHz, 1·2·6채널. WMA·MP3는 스테레오로 줄이고
/// 원본 빈도를 못 받으면 48·44.1kHz 중 되는 것. 다운믹스·리샘플은 Source Reader가 한다
unsafe fn audio_target(sub: &GUID, rate: u32, ch: u32) -> (u32, u32) {
    if *sub == MFAudioFormat_AAC {
        let rate = if rate == 44_100 { rate } else { 48_000 };
        let ch = if matches!(ch, 1 | 2 | 6) { ch } else { 2 };
        return (rate, ch);
    }
    let ch = ch.clamp(1, 2);
    let rate = [rate, 48_000, 44_100]
        .into_iter()
        .find(|&r| unsafe { pick_audio(sub, r, ch) }.is_some())
        .unwrap_or(rate);
    (rate, ch)
}

/// Source Reader가 끼운 영상 처리기의 프레임레이트 변환을 끈다: 켜져 있으면 머리 정보의 초당 프레임(MKV·WebM은 절반으로
/// 보고하기도 한다)에 맞춰 프레임을 버린다. 처리기가 없거나(크기·색 변환이 필요 없을 때) 설정을 못 하면 그대로 둔다
unsafe fn disable_frc(reader: &IMFSourceReader) {
    let Ok(ex) = reader.cast::<IMFSourceReaderEx>() else {
        return;
    };
    for i in 0.. {
        let (mut cat, mut mft) = (GUID::zeroed(), None);
        if unsafe { ex.GetTransformForStream(FIRST_VIDEO, i, Some(&mut cat), &mut mft) }.is_err() {
            break;
        }
        if cat == MFT_CATEGORY_VIDEO_PROCESSOR
            && let Some(attrs) = mft.and_then(|m| unsafe { m.GetAttributes() }.ok())
        {
            let _ = unsafe { attrs.SetUINT32(&MF_XVP_DISABLE_FRC, 1) };
        }
    }
}

/// WMA·MP3 인코더는 정해진 형식만 받는다: 표본 빈도·채널이 같고 128kbps에 가장 가까운 것
unsafe fn pick_audio(subtype: &GUID, rate: u32, ch: u32) -> Option<IMFMediaType> {
    unsafe {
        let list =
            MFTranscodeGetAudioOutputAvailableTypes(subtype, MFT_ENUM_FLAG_ALL.0 as u32, None)
                .ok()?;
        let n = list.GetElementCount().ok()?;
        (0..n)
            .filter_map(|i| list.GetElement(i).ok()?.cast::<IMFMediaType>().ok())
            .filter(|t| {
                t.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).ok() == Some(rate)
                    && t.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).ok() == Some(ch)
            })
            .min_by_key(|t| {
                let bytes = t.GetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND).unwrap_or(0);
                bytes.abs_diff(AUDIO_BPS / 8)
            })
    }
}

/// `src`를 `plan`대로 다시 인코딩해 `dst`에 쓴다. 진행률은 영상(소리만이면 소리) 시간 기준.
/// 그래픽카드 인코더가 있으면 먼저 쓰고, 실패하면(Intel Quick Sync는 64×48 같은 작은 크기에서 마무리 중 실패) Windows 소프트웨어 인코더로 한 번 더
pub fn transcode(
    src: &Path,
    dst: &Path,
    probe: &Probe,
    plan: &Plan,
    ctx: &JobContext,
) -> Result<(), ToolError> {
    match transcode_with(src, dst, probe, plan, ctx, true) {
        Err(e) if !matches!(e, ToolError::Cancelled) => {
            transcode_with(src, dst, probe, plan, ctx, false)
        }
        r => r,
    }
}

fn transcode_with(
    src: &Path,
    dst: &Path,
    probe: &Probe,
    plan: &Plan,
    ctx: &JobContext,
    hardware: bool,
) -> Result<(), ToolError> {
    let _s = Session::new()?;
    let reader = open_reader(src, true)?;
    let video = plan.format.has_video();
    unsafe {
        reader
            .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
            .map_err(|_| unreadable())?;
        let frame = (plan.width, plan.height);
        let raw = if video {
            reader
                .SetStreamSelection(FIRST_VIDEO, true)
                .map_err(|_| unreadable())?;
            let t = video_type(&MFVideoFormat_NV12, frame.0, frame.1, probe.fps, CODEC)?;
            // 초당 프레임을 지정하면 Source Reader가 프레임을 버리거나 복제해 맞춘다 (MKV의 잘못된 머리 정보, VFR 영상).
            // 원본 프레임·시각을 그대로 받고, 인코더에는 잰 초당 프레임을 알려 준다
            t.DeleteItem(&MF_MT_FRAME_RATE).map_err(fail(CODEC))?;
            reader
                .SetCurrentMediaType(FIRST_VIDEO, None, &t)
                .map_err(fail(CODEC))?;
            disable_frc(&reader);
            let cur = reader
                .GetCurrentMediaType(FIRST_VIDEO)
                .map_err(fail(CODEC))?;
            let raw = MFCreateMediaType().map_err(fail(CODEC))?;
            cur.CopyAllItems(&raw).map_err(fail(CODEC))?;
            raw.SetUINT64(&MF_MT_FRAME_RATE, pack(probe.fps.0, probe.fps.1))
                .map_err(fail(CODEC))?;
            Some(raw)
        } else {
            None
        };

        let pcm = if plan.audio {
            reader
                .SetStreamSelection(FIRST_AUDIO, true)
                .map_err(|_| unreadable())?;
            let native = reader
                .GetNativeMediaType(FIRST_AUDIO, 0)
                .map_err(fail(CODEC))?;
            let (rate, ch) = audio_target(
                &plan.format.audio().0,
                native
                    .GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)
                    .unwrap_or(48_000),
                native.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).unwrap_or(2),
            );
            let t = MFCreateMediaType().map_err(fail(CODEC))?;
            t.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate)
                .map_err(fail(CODEC))?;
            t.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, ch)
                .map_err(fail(CODEC))?;
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
                .map_err(fail(CODEC))?;
            t.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)
                .map_err(fail(CODEC))?;
            t.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)
                .map_err(fail(CODEC))?;
            reader
                .SetCurrentMediaType(FIRST_AUDIO, None, &t)
                .map_err(fail(if video {
                    "이 영상의 오디오를 읽을 수 없습니다. '소리 유지'를 끄고 다시 해 보세요."
                } else {
                    "이 영상의 오디오를 읽을 수 없습니다."
                }))?;
            Some(
                reader
                    .GetCurrentMediaType(FIRST_AUDIO)
                    .map_err(fail(CODEC))?,
            )
        } else {
            None
        };

        let writer = open_writer(dst, &plan.format.container(), hardware)?;

        let vout = match &raw {
            Some(raw) => {
                let h264 = plan.format == Format::Mp4;
                let sub = if h264 {
                    MFVideoFormat_H264
                } else {
                    MFVideoFormat_WMV3
                };
                let out = video_type(&sub, frame.0, frame.1, probe.fps, SAVE)?;
                out.SetUINT32(&MF_MT_AVG_BITRATE, plan.video_bps)
                    .map_err(fail(SAVE))?;
                if h264 {
                    out.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)
                        .map_err(fail(SAVE))?;
                }
                // ponytail: 휴대폰 영상의 회전 정보를 그대로 넘긴다. 저장기가 무시하는 Windows 버전이면 눕혀 보일 수 있다
                if let Ok(rot) = reader
                    .GetNativeMediaType(FIRST_VIDEO, 0)
                    .and_then(|t| t.GetUINT32(&MF_MT_VIDEO_ROTATION))
                {
                    let _ = out.SetUINT32(&MF_MT_VIDEO_ROTATION, rot);
                }
                let i = writer.AddStream(&out).map_err(fail(SAVE))?;
                let params = cbr(plan.video_bps)?;
                if h264 {
                    // B 프레임을 쓰면 mp4 저장기가 프레임 간격이 다른(VFR, 휴대폰·화면 녹화) 영상의 시각을 밀어 적는다 (ctts)
                    params
                        .SetUINT32(&CODECAPI_AVEncMPVDefaultBPictureCount, 0)
                        .map_err(fail(SAVE))?;
                }
                writer
                    .SetInputMediaType(i, raw, &params)
                    .map_err(fail(SAVE))?;
                Some(i)
            }
            None => None,
        };

        let aout = match &pcm {
            Some(pcm) => {
                let rate = pcm.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).unwrap_or(0);
                let ch = pcm.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).unwrap_or(0);
                let (sub, name) = plan.format.audio();
                let hint = if video {
                    " '소리 유지'를 끄고 다시 해 보세요."
                } else {
                    ""
                };
                let audio_fail = || {
                    ToolError::Failed(format!(
                        "오디오({rate}Hz, {ch}채널)를 {name}로 바꿀 수 없습니다.{hint}"
                    ))
                };
                let out = if sub == MFAudioFormat_AAC {
                    let aac = MFCreateMediaType().map_err(fail(SAVE))?;
                    aac.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
                        .map_err(fail(SAVE))?;
                    aac.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC)
                        .map_err(fail(SAVE))?;
                    aac.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)
                        .map_err(fail(SAVE))?;
                    aac.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate)
                        .map_err(fail(SAVE))?;
                    aac.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, ch)
                        .map_err(fail(SAVE))?;
                    aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, AUDIO_BPS / 8)
                        .map_err(fail(SAVE))?;
                    aac
                } else {
                    pick_audio(&sub, rate, ch).ok_or_else(audio_fail)?
                };
                let i = writer.AddStream(&out).map_err(|_| audio_fail())?;
                writer
                    .SetInputMediaType(i, pcm, None)
                    .map_err(|_| audio_fail())?;
                Some(i)
            }
            None => None,
        };

        writer.BeginWriting().map_err(fail(SAVE))?;

        // 실제 스트림 번호 → 출력 스트림
        let mut targets: Vec<(u32, u32)> = vec![];
        let mut i = 0;
        while let Ok(selected) = reader.GetStreamSelection(i) {
            if selected.as_bool() {
                let major = reader
                    .GetNativeMediaType(i, 0)
                    .and_then(|t| t.GetGUID(&MF_MT_MAJOR_TYPE))
                    .unwrap_or(GUID::zeroed());
                if let (true, Some(v)) = (major == MFMediaType_Video, vout) {
                    targets.push((i, v));
                } else if let (true, Some(a)) = (major == MFMediaType_Audio, aout) {
                    targets.push((i, a));
                }
            }
            i += 1;
        }
        // 진행률을 셀 스트림: 영상, 없으면 소리
        let clock = vout.or(aout);

        let mut open = targets.len();
        let read = READ;
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
                writer.WriteSample(out, sample).map_err(fail(SAVE))?;
                if Some(out) == clock && probe.duration > 0 {
                    ctx.progress(ts as f32 / probe.duration as f32);
                }
            } else if flags & MF_SOURCE_READERF_STREAMTICK.0 as u32 != 0 {
                writer.SendStreamTick(out, ts).map_err(fail(SAVE))?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                open -= 1;
            }
        }
        writer.Finalize().map_err(fail(SAVE))?;
    }
    Ok(())
}
