//! SVG → PNG (resvg). 글자는 시스템 폰트로 그리고, 외부 파일·주소의 이미지는 읽지 않는다.

use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};
use waple_tools_core::ToolError;
use waple_tools_image_common::{MAX_PIXELS, ensure_pixels, optimize_png};

use crate::optimize::{broken, check_entities};

pub enum Size {
    Scale(f32),
    Width(u32),
}

/// 시스템 폰트 목록은 프로세스에서 한 번만 읽는다 (Windows 폰트 폴더 훑기는 느리다)
fn fonts() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

/// data: 이미지가 상한 안인지. 래스터는 헤더의 가로·세로만 읽고, 텍스트(SVG)는 엔티티 폭탄을 본다
pub fn data_ok(data: &[u8], max_pixels: u64) -> bool {
    if let Ok(text) = std::str::from_utf8(data) {
        return check_entities(text).is_ok();
    }
    let dims = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok());
    // 헤더를 못 읽으면 resvg도 그리지 못한다
    dims.is_none_or(|(w, h)| u64::from(w) * u64::from(h) <= max_pixels)
}

pub fn parse(bytes: &[u8]) -> Result<usvg::Tree, ToolError> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        check_entities(text)?;
    }
    let data = usvg::ImageHrefResolver::default_data_resolver();
    let opt = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            // 안에 넣은 이미지도 크기 검사 없이 풀면 수 GB를 할당할 수 있다
            resolve_data: Box::new(move |mime, d, o| {
                data_ok(&d, MAX_PIXELS).then(|| data(mime, d, o)).flatten()
            }),
            // 파일 경로·URL로 가리킨 이미지는 읽지 않는다 (SVG 안에 넣은 data: 이미지만)
            resolve_string: Box::new(|_, _| None),
        },
        fontdb: fonts(),
        ..Default::default()
    };
    usvg::Tree::from_data(bytes, &opt).map_err(|e| match e {
        usvg::Error::InvalidSize => ToolError::Failed(
            "SVG 크기를 알 수 없습니다 (width·height·viewBox가 없거나 0입니다).".into(),
        ),
        _ => broken(),
    })
}

/// 투명(`bg` 없음) 또는 단색 배경의 PNG
pub fn render_png(
    tree: &usvg::Tree,
    size: Size,
    bg: Option<[u8; 3]>,
) -> Result<Vec<u8>, ToolError> {
    let s = tree.size();
    let (w, h) = match size {
        Size::Scale(k) => (s.width() * k, s.height() * k),
        Size::Width(w) => (w as f32, s.height() * w as f32 / s.width()),
    };
    let (w, h) = (w.round().max(1.0) as u32, h.round().max(1.0) as u32);
    ensure_pixels(w, h)?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(broken)?;
    if let Some([r, g, b]) = bg {
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    }
    let t = tiny_skia::Transform::from_scale(w as f32 / s.width(), h as f32 / s.height());
    resvg::render(tree, t, &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|_| broken())?;
    optimize_png(&png, 2)
}
