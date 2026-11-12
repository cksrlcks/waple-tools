use super::testkit::{make_ramp, ramp_level};
use super::*;

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
