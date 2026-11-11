//! PDF → 이미지 (추가 설치, Generator): 페이지마다 JPG/PNG 한 장 → `<이름>/page-001.jpg` …

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{Fmt, Quality, encode, ensure_pixels};
use waple_tools_pdf_common::{
    PDF_EXTS, out_path, pages_in, parse_options, parse_ranges, schema_of, stem,
};
use waple_tools_pdf_render as render;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Format {
    /// JPG
    #[default]
    Jpg,
    /// PNG
    Png,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Dpi {
    /// 72 (화면 미리보기)
    D72,
    /// 150 (화면용)
    #[default]
    D150,
    /// 300 (인쇄용)
    D300,
    /// 직접 입력
    Custom,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-single-input" = true,
    // 작업대에 페이지 썸네일을 보여 주고, 고른 쪽을 `pages` 옵션에 적는다
    "x-page-picker" = "pages",
    "x-preview" = "none"
))]
struct ToImageOptions {
    /// 이미지 형식
    format: Format,
    /// 해상도(DPI)
    ///
    /// 1인치에 들어갈 픽셀 수입니다. A4 한 쪽이 150이면 1240×1754px, 300이면 2480×3508px입니다.
    dpi: Dpi,
    /// DPI 직접 입력
    #[schemars(range(min = 36, max = 600))]
    #[schemars(extend("x-enabled-when" = { "dpi": ["custom"] }))]
    custom_dpi: u32,
    /// JPG 품질
    #[schemars(range(min = 1, max = 100))]
    #[schemars(extend("x-enabled-when" = { "format": ["jpg"] }))]
    quality: u8,
    /// 페이지 범위
    ///
    /// 비워 두면 전체 페이지입니다. 예: 1-3,5,8- (8- 는 8쪽부터 끝까지). 작업대의 썸네일을 눌러 고를 수도 있습니다.
    pages: String,
}

impl Default for ToImageOptions {
    fn default() -> Self {
        Self {
            format: Format::Jpg,
            dpi: Dpi::D150,
            custom_dpi: 200,
            quality: 90,
            pages: String::new(),
        }
    }
}

impl ToImageOptions {
    fn dpi(&self) -> f32 {
        match self.dpi {
            Dpi::D72 => 72.0,
            Dpi::D150 => 150.0,
            Dpi::D300 => 300.0,
            Dpi::Custom => self.custom_dpi.clamp(36, 600) as f32,
        }
    }
}

struct PdfToImage;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfToImage)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for PdfToImage {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.to_image".into(),
            name: "PDF → 이미지".into(),
            category: "pdf".into(),
            description: "PDF 페이지를 JPG·PNG 이미지로 저장합니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Generator
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<ToImageOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: ToImageOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("PDF를 넣어 주세요.".into()))?;
        let pdf = render::open_file(src)?;
        if render::wants_thumbnails(opts) {
            return render::thumbnails(ctx, &pdf, out_dir);
        }
        let total = pdf.pages().len() as u32;
        let pages = pages_in(&parse_ranges(&o.pages, total)?);
        let scale = o.dpi() / 72.0;
        let (ext, fmt) = match o.format {
            Format::Jpg => ("jpg", Fmt::Jpeg),
            Format::Png => ("png", Fmt::Png),
        };
        let digits = total.to_string().len().max(3);
        let top = PathBuf::from(stem(src));
        let mut names = Vec::with_capacity(pages.len());
        let mut r = render::Renderer::new();
        for (i, &p) in pages.iter().enumerate() {
            ctx.check_cancel()?;
            let page = &pdf.pages()[(p - 1) as usize];
            let (w, h) = render::pixel_size(page, scale);
            if w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
                return Err(ToolError::Failed(format!(
                    "{p}쪽이 너무 커서 이 해상도로 만들 수 없습니다. DPI를 낮춰 주세요."
                )));
            }
            ensure_pixels(w, h)?;
            let img = r.render(page, scale);
            let data = match fmt {
                Fmt::Png => render::png(&img)?,
                _ => encode(
                    &img,
                    fmt,
                    Quality {
                        quality: o.quality.clamp(1, 100),
                        lossless: false,
                    },
                )?,
            };
            let rel = top.join(format!("page-{p:0digits$}.{ext}"));
            std::fs::write(out_path(out_dir, &rel)?, data)?;
            names.push(rel);
            ctx.progress((i + 1) as f32 / pages.len() as f32);
        }
        Ok(ToolOutput::Files(names))
    }
}
