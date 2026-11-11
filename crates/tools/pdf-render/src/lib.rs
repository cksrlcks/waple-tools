//! PDF 페이지 그리기 공용 (hayro): 열기 · 쪽 렌더링 · `x-page-picker`용 썸네일.
//! 글꼴이 들어 있지 않은 한글(CJK) PDF는 Windows 맑은 고딕을 대체 글꼴로 쓴다.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::font::{FontData, FontQuery};
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::page::Page;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use image::{DynamicImage, RgbaImage};
use waple_tools_core::{JobContext, ToolError, ToolOutput};
use waple_tools_image_common::{Fmt, Quality, encode};
use waple_tools_pdf_common::{encrypted, is_encrypted, looks_like_pdf, out_path, unreadable};

/// 파일을 읽어 연다. 암호·손상 PDF는 한국어 오류
pub fn open_file(path: &Path) -> Result<Pdf, ToolError> {
    let bytes = std::fs::read(path)?;
    if !looks_like_pdf(&bytes) {
        return Err(unreadable());
    }
    if is_encrypted(&bytes) {
        return Err(encrypted());
    }
    open(bytes)
}

pub fn open(bytes: Vec<u8>) -> Result<Pdf, ToolError> {
    use hayro::hayro_syntax::LoadPdfError;
    match Pdf::new(bytes) {
        Ok(pdf) if !pdf.pages().is_empty() => Ok(pdf),
        Ok(_) => Err(ToolError::Failed("페이지가 없는 PDF입니다.".into())),
        Err(LoadPdfError::Decryption(_)) => Err(encrypted()),
        Err(LoadPdfError::Invalid) => Err(unreadable()),
    }
}

/// 이 배율로 그렸을 때 픽셀 크기 (hayro와 같은 계산)
pub fn pixel_size(page: &Page, scale: f32) -> (u32, u32) {
    let (w, h) = page.render_dimensions();
    ((w * scale) as u32, (h * scale) as u32)
}

/// `%WINDIR%\Fonts\malgun.ttf` (굵게는 malgunbd.ttf). 없으면 None — 그때는 hayro 기본 글꼴
fn malgun(bold: bool) -> Option<(FontData, u32)> {
    static REGULAR: OnceLock<Option<FontData>> = OnceLock::new();
    static BOLD: OnceLock<Option<FontData>> = OnceLock::new();
    let (cell, file) = if bold {
        (&BOLD, "malgunbd.ttf")
    } else {
        (&REGULAR, "malgun.ttf")
    };
    cell.get_or_init(|| {
        let dir = std::env::var_os("WINDIR")?;
        let bytes = std::fs::read(std::path::Path::new(&dir).join("Fonts").join(file)).ok()?;
        Some(Arc::new(bytes) as FontData)
    })
    .clone()
    .map(|d| (d, 0))
}

fn settings() -> InterpreterSettings {
    InterpreterSettings {
        font_resolver: Arc::new(|query| match query {
            FontQuery::Standard(s) => Some(s.get_font_data()),
            FontQuery::Fallback(f) => {
                let cjk = f.character_collection.is_some();
                cjk.then(|| malgun(f.is_bold))
                    .flatten()
                    .or_else(|| Some(f.pick_standard_font().get_font_data()))
            }
        }),
        ..Default::default()
    }
}

pub struct Renderer {
    settings: InterpreterSettings,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            settings: settings(),
        }
    }

    /// 흰 배경에 그린 RGBA (불투명)
    pub fn render(&mut self, page: &Page, scale: f32) -> DynamicImage {
        let cache = RenderCache::new();
        let pixmap = hayro::render(
            page,
            &cache,
            &self.settings,
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let (w, h) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        // 배경이 불투명이라 premultiplied = 보통 RGBA
        let rgba = RgbaImage::from_raw(w, h, pixmap.data_as_u8_slice().to_vec())
            .unwrap_or_else(|| RgbaImage::new(w.max(1), h.max(1)));
        DynamicImage::ImageRgba8(rgba)
    }
}

/// 빠른 PNG (페이지 이미지는 크므로 oxipng 최적화는 하지 않는다)
pub fn png(img: &DynamicImage) -> Result<Vec<u8>, ToolError> {
    let mut out = Vec::new();
    DynamicImage::ImageRgb8(img.to_rgb8())
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|_| ToolError::Failed("이미지를 저장할 수 없습니다.".into()))?;
    Ok(out)
}

/// 이 키가 옵션에 있으면 도구는 본 작업 대신 쪽마다 작은 썸네일을 만든다 (앱의 `x-page-picker` 화면이 부른다)
pub const THUMBNAILS: &str = "x-thumbnails";
/// 썸네일 긴 변(px)
const THUMB_SIDE: f32 = 240.0;

pub fn wants_thumbnails(opts: &serde_json::Value) -> bool {
    opts.get(THUMBNAILS).is_some()
}

/// `thumbs/p-0001.jpg` … 모든 쪽, 순서대로
pub fn thumbnails(ctx: &JobContext, pdf: &Pdf, out_dir: &Path) -> Result<ToolOutput, ToolError> {
    let pages = pdf.pages();
    let mut r = Renderer::new();
    let mut names = Vec::with_capacity(pages.len());
    for (i, page) in pages.iter().enumerate() {
        ctx.check_cancel()?;
        let (w, h) = page.render_dimensions();
        let img = r.render(page, THUMB_SIDE / w.max(h).max(1.0));
        let q = Quality {
            quality: 75,
            lossless: false,
        };
        let rel = PathBuf::from("thumbs").join(format!("p-{:04}.jpg", i + 1));
        std::fs::write(out_path(out_dir, &rel)?, encode(&img, Fmt::Jpeg, q)?)?;
        names.push(rel);
        ctx.progress((i + 1) as f32 / pages.len() as f32);
    }
    Ok(ToolOutput::Files(names))
}
