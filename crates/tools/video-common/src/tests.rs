use super::testkit::{make_ramp, ramp_level};
use super::*;
use crate::mf::{self, measured_fps};

/// 위 절반·아래 절반 평균 밝기 (R 기준)
fn halves(f: &image::RgbaImage, w: u32, h: u32) -> (f64, f64) {
    let row = (w * 4) as usize;
    let avg = |rows: std::ops::Range<u32>| {
        let n = rows.len() as f64 * w as f64;
        rows.flat_map(|y| f.as_raw()[y as usize * row..(y as usize + 1) * row].chunks(4))
            .map(|p| p[0] as f64)
            .sum::<f64>()
            / n
    };
    (avg(0..h / 2 - 2), avg(h / 2 + 2..h))
}

#[test]
fn frames_are_upright_scaled_and_seekable() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("ramp.mp4");
    make_ramp(&src, 320, 240, 4);
    let p = probe(&src).unwrap();
    let mut r = FrameReader::open(&src, &p, 160, 120).unwrap();
    let first = r.next_frame().unwrap().unwrap();
    assert_eq!(r.image(&first).unwrap().dimensions(), (160, 120));
    assert_eq!(first.time, 0);
    r.seek(30_000_000).unwrap();
    let f = loop {
        let f = r.next_frame().unwrap().unwrap();
        if f.time + 333_333 > 30_000_000 {
            break f;
        }
    };
    let (top, bottom) = halves(&r.image(&f).unwrap(), 160, 120);
    let want = ramp_level(4, f.time as f64 / 1e7);
    assert!((top - want).abs() < 12.0, "위 {top}, 기대 {want}");
    assert!(bottom < 12.0, "아래는 검정 (뒤집히지 않음): {bottom}");
    while r.next_frame().unwrap().is_some() {}
}

#[test]
fn writer_round_trips_rgba_with_variable_delays() {
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("out.mp4");
    let (w, h) = (64, 48);
    let wr = Mp4Writer::create(&dst, w, h, (10, 1), 1_000_000).unwrap();
    // 흰 위·검정 아래, 0.1초 · 0.5초 · 0.4초
    let mut img = vec![0u8; (w * h * 4) as usize];
    for (i, p) in img.chunks_mut(4).enumerate() {
        let v = if (i as u32 / w) < h / 2 { 255 } else { 0 };
        p.copy_from_slice(&[v, v, v, 255]);
    }
    let mut t = 0;
    for d in [1_000_000, 5_000_000, 4_000_000] {
        wr.write(&img, t, d).unwrap();
        t += d;
    }
    wr.finish().unwrap();
    let p = probe(&dst).unwrap();
    assert_eq!((p.width, p.height), (w, h));
    assert!(
        (p.duration as f64 / 1e7 - 1.0).abs() < 0.15,
        "{}",
        p.duration
    );
    let mut r = FrameReader::open(&dst, &p, w, h).unwrap();
    let mut times = vec![];
    let first = r.next_frame().unwrap().unwrap();
    times.push(first.time);
    while let Some(f) = r.next_frame().unwrap() {
        times.push(f.time);
    }
    assert_eq!(
        times,
        vec![0, 1_000_000, 6_000_000],
        "프레임마다 다른 시각 그대로"
    );
    let (top, bottom) = halves(&r.image(&first).unwrap(), w, h);
    assert!(top > 230.0 && bottom < 25.0, "{top} {bottom}");
}

/// 프레임 간격이 다른(VFR) 64×48 영상: 0.1초 · 0.5초 · 0.4초
fn vfr(dst: &std::path::Path) {
    let wr = Mp4Writer::create(dst, 64, 48, (30, 1), 1_000_000).unwrap();
    let mut t = 0;
    for (d, v) in [(1_000_000, 40), (5_000_000, 140), (4_000_000, 240)] {
        wr.write(&[v, v, v, 255].repeat(64 * 48), t, d).unwrap();
        t += d;
    }
    wr.finish().unwrap();
}

fn frame_times(src: &std::path::Path) -> Vec<i64> {
    let p = probe(src).unwrap();
    let mut r = FrameReader::open(src, &p, p.width, p.height).unwrap();
    let mut times = vec![];
    while let Some(f) = r.next_frame().unwrap() {
        times.push(f.time);
    }
    times
}

#[test]
fn transcode_keeps_variable_frame_times() {
    // 예전에는 Source Reader가 머리 정보의 초당 프레임으로 프레임을 맞춰 버리거나 늘렸다
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("vfr.mp4");
    vfr(&src);
    let p = probe(&src).unwrap();
    let ctx = waple_tools_core::JobContext::new(|_| {}, Default::default());
    for format in [Format::Mp4, Format::Wmv] {
        let dst = dir.path().join(format!("out.{}", format.ext()));
        let plan = Plan {
            format,
            width: 64,
            height: 48,
            video_bps: 500_000,
            audio: false,
        };
        transcode(&src, &dst, &p, &plan, &ctx).unwrap();
        let q = probe(&dst).unwrap();
        assert!(
            (q.duration as f64 / 1e7 - 1.0).abs() < 0.1,
            "{:?} {q:?}",
            format
        );
        let times = frame_times(&dst);
        assert_eq!(times.len(), 3, "{format:?} {times:?}");
        assert!(
            (times[1] - 1_000_000).abs() < 20_000,
            "{format:?} {times:?}"
        );
        assert!(
            (times[2] - 6_000_000).abs() < 20_000,
            "{format:?} {times:?}"
        );
    }
}

#[test]
fn rotation_turns_frames_clockwise() {
    // MF 회전 90 = 보여 줄 때 시계 방향으로 90도 (ffmpeg의 display matrix -90, 휴대폰 세로 영상).
    // 저장된 왼쪽 위 흰 구석은 90이면 오른쪽 위, 270이면 왼쪽 아래로 간다
    let dir = tempfile::tempdir().unwrap();
    for (rot, corner) in [(0, (0, 0)), (90, (1, 0)), (180, (1, 1)), (270, (0, 1))] {
        let src = dir.path().join(format!("r{rot}.mp4"));
        super::testkit::make_marked(&src, 320, 240, rot);
        let p = probe(&src).unwrap();
        assert_eq!(p.rotation, rot);
        let (w, h) = p.shown_size();
        assert_eq!(
            (w, h),
            if rot % 180 == 90 {
                (240, 320)
            } else {
                (320, 240)
            }
        );
        let mut r = FrameReader::open(&src, &p, w / 2, h / 2).unwrap();
        let f = r.next_frame().unwrap().unwrap();
        let img = r.image(&f).unwrap();
        assert_eq!(img.dimensions(), (w / 2, h / 2));
        // 네 구석 중 흰 곳
        let at = |cx: u32, cy: u32| img.get_pixel(4 + cx * (w / 2 - 9), 4 + cy * (h / 2 - 9))[0];
        for cx in 0..2 {
            for cy in 0..2 {
                let white = at(cx, cy) > 200;
                assert_eq!(white, (cx, cy) == corner, "회전 {rot}, 구석 ({cx},{cy})");
            }
        }
    }
}

#[test]
fn fps_from_frame_times_when_header_is_off() {
    // MKV처럼 머리 정보가 절반(15)인데 실제 프레임은 30fps
    let times: Vec<i64> = (0..30).map(|i| i * 333_333).collect();
    let m = measured_fps(&times);
    assert_eq!(mf::pick_fps(Some((15, 1)), m), (30_000, 1000));
    assert_eq!(mf::pick_fps(Some((30, 1)), m), (30, 1), "맞으면 머리 정보");
    assert_eq!(mf::pick_fps(None, None), (30, 1));
    assert_eq!(mf::pick_fps(Some((25, 1)), None), (25, 1));
}

#[test]
fn odd_size_writer_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let e = Mp4Writer::create(&dir.path().join("x.mp4"), 63, 48, (10, 1), 1_000_000)
        .err()
        .unwrap();
    assert!(e.to_string().contains("크기"), "{e}");
    let e = Mp4Writer::create(&dir.path().join("y.mp4"), 32, 48, (10, 1), 1_000_000)
        .err()
        .unwrap();
    assert!(e.to_string().contains("크기"), "{e}");
}

#[test]
fn fit_width_never_upscales_and_is_even() {
    assert_eq!(fit_width(1920, 1080, Some(640)), (640, 360));
    assert_eq!(fit_width(320, 240, Some(640)), (320, 240));
    assert_eq!(fit_width(641, 361, None), (640, 360));
    assert_eq!(fit_width(1080, 1920, Some(480)), (480, 854));
}
