//! 좌표 숫자 반올림: path d와 숫자 목록 속성(points·transform·viewBox 등)

use svgtypes::{PathParser, PathSegment as S};

/// `p`자리 반올림 값
fn round(v: f64, p: u8) -> f64 {
    let k = 10f64.powi(i32::from(p));
    (v * k).round() / k
}

/// 반올림한 값. 0이 아니던 값이 0이 되면 (도형·선·변환이 사라지므로) 원래 값
fn keep(v: f64, p: u8) -> f64 {
    let r = round(v, p);
    if r == 0.0 && v != 0.0 { v } else { r }
}

/// 소수 자릿수: `p`자리, 단 유효숫자 `sig`개는 남긴다 (SVGO transformPrecision처럼)
fn decimals(v: f64, p: u8, sig: u8) -> u8 {
    if sig == 0 || v == 0.0 || !v.is_finite() {
        return p;
    }
    let mag = v.abs().log10().floor() as i32;
    (i32::from(sig) - 1 - mag).clamp(i32::from(p), 15) as u8
}

/// 짧은 표기 (이미 반올림한 값): 앞의 0을 뺀다 (0.5 → .5, -0 → 0)
fn fmt(v: f64) -> String {
    // f64의 Display는 되읽으면 같은 값이 되는 가장 짧은 십진수다 (지수 표기 없음)
    let s = v.to_string();
    let s = s.as_str();
    match s {
        "-0" => "0".into(),
        _ if s.starts_with("0.") => s[1..].into(),
        _ if s.starts_with("-0.") => format!("-{}", &s[2..]),
        _ => s.into(),
    }
}

/// 앞 숫자와 붙어 한 숫자로 읽히지 않게 필요할 때만 띄운다 (`-`나, 앞 숫자에 소수점이 있을 때의 `.` 앞은 안 띄운다)
fn push_num(out: &mut String, s: &str) {
    let prev_has_dot = out
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .ends_with('.');
    let glued = s.starts_with('-') || (s.starts_with('.') && prev_has_dot);
    if !glued && out.ends_with(|c: char| c.is_ascii_digit() || c == '.') {
        out.push(' ');
    }
    out.push_str(s);
}

/// `b[i..]`가 숫자로 시작하면 숫자의 끝 위치
fn number_end(b: &[u8], i: usize) -> Option<usize> {
    let digits = |mut j: usize| {
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        j
    };
    let mut j = i + usize::from(matches!(b.get(i), Some(b'+' | b'-')));
    let int_end = digits(j);
    let mut any = int_end > j;
    j = int_end;
    if b.get(j) == Some(&b'.') {
        let frac_end = digits(j + 1);
        if any || frac_end > j + 1 {
            any = true;
            j = frac_end;
        }
    }
    if !any {
        return None;
    }
    if matches!(b.get(j), Some(b'e' | b'E')) {
        let k = j + 1 + usize::from(matches!(b.get(j + 1), Some(b'+' | b'-')));
        let e_end = digits(k);
        if e_end > k {
            j = e_end;
        }
    }
    Some(j)
}

/// 문자열 안의 소수(1.234, 1e-3)만 반올림한다. 정수·단위·나머지 글자는 그대로.
/// `sig`가 0이 아니면 유효숫자를 그만큼은 남긴다 (transform의 작은 배율이 0이 되지 않게)
pub fn round_numbers(s: &str, p: u8, sig: u8) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if let Some(end) = number_end(b, i) {
            let tok = &s[i..end];
            match tok.parse::<f64>() {
                Ok(v) if v.is_finite() && tok.contains(['.', 'e', 'E']) => {
                    push_num(&mut out, &fmt(keep(v, decimals(v, p, sig))))
                }
                _ => push_num(&mut out, tok),
            }
            i = end;
        } else {
            let c = s[i..].chars().next().unwrap_or_default();
            out.push(c);
            i += c.len_utf8().max(1);
        }
    }
    out
}

/// 한 축의 좌표: (쓸 값, 원래 절대 위치, 쓴 값 기준 절대 위치).
/// 상대 좌표는 "쓴 값으로 도착한 위치" 기준으로 다시 재서 반올림 오차가 쌓이지 않게 한다.
fn axis(abs: bool, v: f64, base: f64, base_out: f64, p: u8) -> (f64, f64, f64) {
    if abs {
        let e = keep(v, p);
        (e, v, e)
    } else {
        let target = base + v;
        let e = match round(target - base_out, p) {
            // 0이 아니던 이동이 0이 되면 원래 값을 쓴다 (아주 작은 도형이 사라지지 않게)
            0.0 if v != 0.0 => v,
            e => e,
        };
        (e, target, base_out + e)
    }
}

/// path d를 다시 쓴다. 읽을 수 없는 path면 None (원래 값을 그대로 둔다)
pub fn round_path(d: &str, p: u8) -> Option<String> {
    let mut out = String::with_capacity(d.len());
    let mut prev = ' ';
    // 원래 좌표 기준 현재점·부분 경로 시작점, 쓴 좌표 기준 현재점·시작점
    let (mut cur, mut cur_out, mut start, mut start_out) =
        ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
    for seg in PathParser::from(d) {
        let seg = seg.ok()?;
        let (bx, by, bxo, byo) = (cur.0, cur.1, cur_out.0, cur_out.1);
        let ax = |abs, v| axis(abs, v, bx, bxo, p);
        let ay = |abs, v| axis(abs, v, by, byo, p);
        let mut nums = Vec::with_capacity(7);
        // 끝점 (x, y)를 쓰고 현재점을 옮긴다
        let mut end = |abs, x, y, nums: &mut Vec<f64>| {
            let (ex, ox, qx) = ax(abs, x);
            let (ey, oy, qy) = ay(abs, y);
            nums.extend([ex, ey]);
            cur = (ox, oy);
            cur_out = (qx, qy);
        };
        let (letter, abs) = match seg {
            S::MoveTo { abs, x, y } => {
                end(abs, x, y, &mut nums);
                start = cur;
                start_out = cur_out;
                ('M', abs)
            }
            S::LineTo { abs, x, y } => {
                end(abs, x, y, &mut nums);
                ('L', abs)
            }
            S::HorizontalLineTo { abs, x } => {
                let (e, o, q) = ax(abs, x);
                nums.push(e);
                (cur.0, cur_out.0) = (o, q);
                ('H', abs)
            }
            S::VerticalLineTo { abs, y } => {
                let (e, o, q) = ay(abs, y);
                nums.push(e);
                (cur.1, cur_out.1) = (o, q);
                ('V', abs)
            }
            S::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                nums.extend([ax(abs, x1).0, ay(abs, y1).0, ax(abs, x2).0, ay(abs, y2).0]);
                end(abs, x, y, &mut nums);
                ('C', abs)
            }
            S::SmoothCurveTo { abs, x2, y2, x, y } => {
                nums.extend([ax(abs, x2).0, ay(abs, y2).0]);
                end(abs, x, y, &mut nums);
                ('S', abs)
            }
            S::Quadratic { abs, x1, y1, x, y } => {
                nums.extend([ax(abs, x1).0, ay(abs, y1).0]);
                end(abs, x, y, &mut nums);
                ('Q', abs)
            }
            S::SmoothQuadratic { abs, x, y } => {
                end(abs, x, y, &mut nums);
                ('T', abs)
            }
            S::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                nums.extend([
                    keep(rx, p),
                    keep(ry, p),
                    keep(x_axis_rotation, p),
                    f64::from(u8::from(large_arc)),
                    f64::from(u8::from(sweep)),
                ]);
                end(abs, x, y, &mut nums);
                ('A', abs)
            }
            S::ClosePath { abs } => {
                cur = start;
                cur_out = start_out;
                ('Z', abs)
            }
        };
        let letter = if abs {
            letter
        } else {
            letter.to_ascii_lowercase()
        };
        // 같은 명령이 이어지면 글자를 생략한다. M 뒤 생략은 L로 읽히므로 M은, 숫자 없는 Z는 늘 쓴다
        if letter != prev || letter.eq_ignore_ascii_case(&'m') || nums.is_empty() {
            out.push(letter);
        }
        prev = letter;
        for n in nums {
            if !n.is_finite() {
                return None;
            }
            push_num(&mut out, &fmt(n));
        }
    }
    Some(out)
}
