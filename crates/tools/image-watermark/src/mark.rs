//! 워터마크 한 장(RGBA) 만들기: 텍스트 래스터라이즈 · 로고 크기 맞춤

use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontVec, OutlinedGlyph, PxScale, ScaleFont, point};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use waple_tools_core::ToolError;
use waple_tools_image_common::{ensure_pixels, load, resize_rgba};

/// 로고를 읽은 뒤 긴 변을 이 크기까지 줄여 둔다
const MAX_LOGO_SIDE: u32 = 4096;

/// Windows 글꼴 폴더에서 글꼴을 읽는다 (TTC는 첫 글꼴)
pub(crate) fn system_font(file: &str) -> Result<FontVec, ToolError> {
    let dir = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("Fonts");
    let bytes = std::fs::read(dir.join(file)).map_err(|_| {
        ToolError::Failed(format!(
            "글꼴 파일({file})을 찾을 수 없습니다. 다른 글꼴을 골라 주세요."
        ))
    })?;
    FontVec::try_from_vec_and_index(bytes, 0)
        .map_err(|_| ToolError::Failed(format!("글꼴 파일({file})을 읽을 수 없습니다.")))
}

/// 로고 파일을 읽는다 (EXIF 방향 반영)
pub(crate) fn load_logo(path: &str) -> Result<RgbaImage, ToolError> {
    if path.trim().is_empty() {
        return Err(ToolError::InvalidOptions("로고 파일을 골라 주세요.".into()));
    }
    let p = Path::new(path);
    if !p.is_file() {
        return Err(ToolError::Failed("로고 파일을 찾을 수 없습니다.".into()));
    }
    // 원래 이유(읽을 수 없음, 움직이는 GIF 등)를 살린다
    let l = load(p).map_err(|e| ToolError::Failed(format!("로고 파일: {e}")))?;
    let (w, h) = (l.image.width(), l.image.height());
    ensure_pixels(w, h)?;
    let logo = l.image.to_rgba8();
    // 워터마크로는 이보다 클 필요가 없다: 뒤에서 크기를 바꾸는 비용을 줄인다
    let s = fit_scale(w as f32, h as f32, MAX_LOGO_SIDE, MAX_LOGO_SIDE);
    Ok(if s < 1.0 {
        let size = |v: u32| ((v as f32 * s).round() as u32).max(1);
        resize_rgba(&logo, size(w), size(h), FilterType::Lanczos3)
    } else {
        logo
    })
}

/// (w, h)가 (max_w, max_h) 안에 들어가게 하는 배율 (1 이하)
fn fit_scale(w: f32, h: f32, max_w: u32, max_h: u32) -> f32 {
    (max_w as f32 / w.max(1.0))
        .min(max_h as f32 / h.max(1.0))
        .min(1.0)
}

/// 한 줄로 배치한 글리프 외곽선과 그 픽셀 경계 (min x, min y, max x, max y)
pub(crate) fn layout(font: &FontVec, text: &str, px: f32) -> (Vec<OutlinedGlyph>, [f32; 4]) {
    let scale = PxScale::from(px);
    let sf = font.as_scaled(scale);
    let mut x = 0.0;
    let mut prev = None;
    let mut glyphs = Vec::new();
    for c in text.chars().filter(|c| !c.is_control()) {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            x += sf.kern(p, id);
        }
        if let Some(g) =
            font.outline_glyph(id.with_scale_and_position(scale, point(x, sf.ascent())))
        {
            glyphs.push(g);
        }
        x += sf.h_advance(id);
        prev = Some(id);
    }
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for g in &glyphs {
        let r = g.px_bounds();
        b = [
            b[0].min(r.min.x),
            b[1].min(r.min.y),
            b[2].max(r.max.x),
            b[3].max(r.max.y),
        ];
    }
    (glyphs, b)
}

/// 칸에 들어가는 글자 높이. 그릴 글리프가 없으면 None
pub(crate) fn fit_px(font: &FontVec, text: &str, px: f32, max_w: u32, max_h: u32) -> Option<f32> {
    let (_, b) = layout(font, text, px.max(1.0));
    if b[0] > b[2] {
        return None;
    }
    // 픽셀 경계는 양끝을 바깥으로 반올림해 줄인 뒤에도 최대 2px 넓어질 수 있다
    let slack = |m: u32| m.saturating_sub(2).max(1);
    let s = fit_scale(b[2] - b[0], b[3] - b[1], slack(max_w), slack(max_h));
    Some((px * s).max(1.0))
}

/// 글자 높이 `px`로 그린 텍스트. 칸보다 크면 글자를 줄여 다시 그린다.
pub(crate) fn text(
    font: &FontVec,
    text: &str,
    px: f32,
    color: [u8; 3],
    max_w: u32,
    max_h: u32,
) -> Result<RgbaImage, ToolError> {
    let px = fit_px(font, text, px, max_w, max_h)
        .ok_or_else(|| ToolError::Failed("텍스트를 그릴 수 없습니다.".into()))?;
    let (glyphs, b) = layout(font, text, px);
    let (w, h) = (
        ((b[2] - b[0]).ceil() as u32).clamp(1, max_w),
        ((b[3] - b[1]).ceil() as u32).clamp(1, max_h),
    );
    let mut img = RgbaImage::from_pixel(w, h, Rgba([color[0], color[1], color[2], 0]));
    for g in &glyphs {
        let r = g.px_bounds();
        let (ox, oy) = ((r.min.x - b[0]) as u32, (r.min.y - b[1]) as u32);
        g.draw(|x, y, cov| {
            let (x, y) = (ox + x, oy + y);
            if x < w && y < h {
                let a = &mut img.get_pixel_mut(x, y).0[3];
                // 겹친 글리프는 진한 쪽
                *a = (*a).max((cov.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        });
    }
    Ok(img)
}

/// 가로 `width`로 줄이거나 키운 로고. 칸보다 크면 맞춰 줄인다.
pub(crate) fn logo(logo: &RgbaImage, width: f32, max_w: u32, max_h: u32) -> RgbaImage {
    let (lw, lh) = (logo.width() as f32, logo.height() as f32);
    let (w, h) = (width, width * lh / lw);
    let s = fit_scale(w, h, max_w, max_h);
    let (tw, th) = (
        ((w * s).round() as u32).max(1),
        ((h * s).round() as u32).max(1),
    );
    resize_rgba(logo, tw, th, FilterType::Lanczos3)
}
