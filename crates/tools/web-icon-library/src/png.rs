//! SVG → PNG (resvg). 글자·외부 이미지는 그리지 않는다. `currentColor`는 검정.

use resvg::{tiny_skia, usvg};
use waple_tools_core::ToolError;

/// 큰 비율의 아이콘 × 4배가 수 GB를 할당하지 않도록 (image-common과 같은 상한)
const MAX_PIXELS: u64 = 100_000_000;

fn fail() -> ToolError {
    ToolError::Failed("PNG를 만들 수 없습니다.".into())
}

/// SVG 크기의 `scale`배 PNG (투명 배경)
pub fn render(svg: &str, scale: u32) -> Result<Vec<u8>, ToolError> {
    let opt = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|_| fail())?;
    let k = scale as f32;
    let s = tree.size();
    let (w, h) = (
        (s.width() * k).round().max(1.0) as u32,
        (s.height() * k).round().max(1.0) as u32,
    );
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(ToolError::Failed(
            "PNG가 너무 큽니다. 크기를 줄여 주세요.".into(),
        ));
    }
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(fail)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(k, k),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|_| fail())
}
