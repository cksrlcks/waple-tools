//! 색 변환 (node-vibrant와 같은 식)

pub type Rgb = [u8; 3];

pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// h·s·l 모두 0~1
pub fn to_hsl(c: Rgb) -> [f64; 3] {
    let [r, g, b] = c.map(|v| f64::from(v) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return [0.0, 0.0, l];
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    [h / 6.0, s, l]
}

pub fn from_hsl([h, s, l]: [f64; 3]) -> Rgb {
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return [v; 3];
    }
    let hue = |p: f64, q: f64, mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [h + 1.0 / 3.0, h, h - 1.0 / 3.0].map(|t| (hue(p, q, t) * 255.0).round() as u8)
}

/// WCAG 상대 휘도 (0~1)
pub fn luminance(c: Rgb) -> f64 {
    let [r, g, b] = c.map(|v| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub const WHITE: Rgb = [255; 3];
pub const BLACK: Rgb = [0; 3];

/// node-vibrant Swatch.bodyTextColor와 같은 식: YIQ 밝기가 150 미만이면 흰색, 아니면 검정
pub fn body_text(bg: Rgb) -> Rgb {
    let [r, g, b] = bg.map(u32::from);
    if (r * 299 + g * 587 + b * 114) < 150_000 {
        WHITE
    } else {
        BLACK
    }
}
