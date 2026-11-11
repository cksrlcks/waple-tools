//! 이미지 → PDF (추가 설치, Combine): 넣은 순서대로 한 장에 한 쪽 → `<첫 이미지 이름>.pdf`

mod page;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::INPUT_EXTS;
use waple_tools_pdf_common::lopdf::{Document, Object, dictionary};
use waple_tools_pdf_common::{out_path, parse_options, save, schema_of, stem};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum PageSize {
    /// A4
    #[default]
    A4,
    /// Letter
    Letter,
    /// 이미지 크기 그대로
    Image,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Orientation {
    /// 자동 (이미지에 맞춤)
    #[default]
    Auto,
    /// 세로
    Portrait,
    /// 가로
    Landscape,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct FromImageOptions {
    /// 페이지 크기
    ///
    /// 이미지 크기 그대로는 96dpi 기준입니다 (1000px → 약 26cm).
    page_size: PageSize,
    /// 방향
    #[schemars(extend("x-enabled-when" = { "pageSize": ["a4", "letter"] }))]
    orientation: Orientation,
    /// 여백(mm)
    ///
    /// 이미지는 비율을 유지한 채 여백 안에 맞춰 가운데에 놓습니다.
    #[schemars(range(min = 0, max = 50))]
    #[schemars(extend("x-enabled-when" = { "pageSize": ["a4", "letter"] }))]
    margin: u8,
    /// 이미지 다시 압축 (JPEG)
    ///
    /// 끄면 JPG는 그대로 넣고 나머지는 손실 없이 넣습니다. 켜면 모두 JPEG로 압축해 용량이 줄어듭니다.
    recompress: bool,
    /// JPEG 품질
    #[schemars(range(min = 30, max = 100))]
    #[schemars(extend("x-enabled-when" = { "recompress": [true] }))]
    quality: u8,
}

impl Default for FromImageOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            orientation: Orientation::Auto,
            margin: 10,
            recompress: false,
            quality: 85,
        }
    }
}

struct PdfFromImage;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfFromImage)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for PdfFromImage {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.from_image".into(),
            name: "이미지 → PDF".into(),
            category: "pdf".into(),
            description: "여러 이미지를 넣은 순서대로 PDF 한 개로 묶습니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Combine
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<FromImageOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: FromImageOptions = parse_options(opts)?;
        let first = input
            .first()
            .ok_or_else(|| ToolError::Failed("이미지를 넣어 주세요.".into()))?;
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids: Vec<Object> = vec![];
        for (i, p) in input.iter().enumerate() {
            ctx.check_cancel()?;
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            let page = page::add(&mut doc, pages_id, p, &o)
                .map_err(|e| ToolError::Failed(format!("{name}: {e}")))?;
            kids.push(page.into());
            ctx.progress((i + 1) as f32 / (input.len() + 1) as f32);
        }
        let count = kids.len() as i64;
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        let rel = PathBuf::from(format!("{}.pdf", stem(first)));
        save(&mut doc, &out_path(out_dir, &rel)?)?;
        ctx.progress(1.0);
        Ok(ToolOutput::Files(vec![rel]))
    }
}
