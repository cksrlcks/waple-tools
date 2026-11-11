//! PDF 압축 (추가 설치, PerFile): 가볍게(객체 정리·스트림 재압축) / 강하게(+ 내부 이미지 다운샘플·JPEG 재압축)

mod images;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_pdf_common::{PDF_EXTS, open, out_path, parse_options, save, schema_of};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Level {
    /// 가볍게 (화질 그대로)
    #[default]
    Light,
    /// 강하게 (이미지 화질을 낮춤)
    Strong,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct CompressOptions {
    /// 압축 수준
    ///
    /// 가볍게: 쓰지 않는 객체를 지우고 압축을 다시 합니다. 강하게: 안에 든 사진을 줄이고 JPEG로 다시 압축합니다. 글자는 어느 쪽이든 그대로 선택할 수 있습니다.
    level: Level,
    /// 이미지 최대 DPI
    ///
    /// 이보다 촘촘한 사진은 이 해상도로 줄입니다. 화면용 150, 인쇄용 300.
    #[schemars(range(min = 72, max = 600))]
    #[schemars(extend("x-enabled-when" = { "level": ["strong"] }))]
    max_dpi: u32,
    /// 이미지 품질
    #[schemars(range(min = 30, max = 95))]
    #[schemars(extend("x-enabled-when" = { "level": ["strong"] }))]
    quality: u8,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            level: Level::Light,
            max_dpi: 150,
            quality: 75,
        }
    }
}

struct PdfCompress;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfCompress)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for PdfCompress {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.compress".into(),
            name: "PDF 압축".into(),
            category: "pdf".into(),
            description: "PDF 용량을 줄입니다. 글자는 그대로 선택할 수 있습니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<CompressOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: CompressOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("PDF를 넣어 주세요.".into()))?;
        let mut doc = open(src)?;
        ctx.check_cancel()?;
        doc.prune_objects();
        doc.delete_zero_length_streams();
        // 사진은 원래 압축된 크기와 비교해야 하므로 먼저
        if o.level == Level::Strong {
            images::shrink(
                &mut doc,
                o.max_dpi.clamp(72, 600),
                o.quality.clamp(30, 95),
                ctx,
            )?;
        }
        // 압축을 풀었다 최고 수준으로 다시 한다 (JPEG 등 못 푸는 것은 그대로)
        images::decompress_all(&mut doc);
        let rel = PathBuf::from(src.file_name().unwrap_or_default());
        save(&mut doc, &out_path(out_dir, &rel)?)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
