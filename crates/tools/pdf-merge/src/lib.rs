//! PDF 병합 (추가 설치, Combine): 넣은 순서대로 PDF 여러 개 → `<첫 파일 이름>-merged.pdf` 하나

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_pdf_common::lopdf::{Bookmark, Document, Object, ObjectId, dictionary};
use waple_tools_pdf_common::{
    PDF_EXTS, drop_page_refs, open, out_path, parse_options, push_down_inherited, save, schema_of,
    stem,
};

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct MergeOptions {
    /// 파일 이름을 북마크로 추가
    ///
    /// 합친 PDF의 북마크(목차)에 파일마다 이름을 넣어 그 파일의 첫 페이지로 가게 합니다. 원래 PDF에 있던 북마크는 넣지 않습니다.
    bookmarks: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self { bookmarks: true }
    }
}

struct PdfMerge;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfMerge)
}

waple_tools_addon_abi::export_tool!(tool);

fn is_type(obj: &Object, ty: &[u8]) -> bool {
    obj.as_dict()
        .ok()
        .and_then(|d| d.get(b"Type").ok())
        .and_then(|t| t.as_name().ok())
        == Some(ty)
}

/// 문서들의 페이지를 순서대로 새 페이지 트리 하나에 모은다.
// ponytail: 양식(AcroForm)·원본 북마크·태그 구조는 버린다. 필요해지면 문서마다 옮겨 붙인다
fn merge(docs: Vec<(String, Document)>, bookmarks: bool) -> Document {
    let mut out = Document::with_version("1.5");
    let pages_id = out.new_object_id();
    let mut kids: Vec<Object> = vec![];
    for (title, mut doc) in docs {
        push_down_inherited(&mut doc);
        drop_page_refs(&mut doc);
        doc.renumber_objects_with(out.max_id + 1);
        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        for (id, mut obj) in std::mem::take(&mut doc.objects) {
            // 원래 문서의 카탈로그·페이지 트리 노드는 버리고 페이지만 새 트리에 단다
            if is_type(&obj, b"Catalog") || is_type(&obj, b"Pages") {
                continue;
            }
            if pages.contains(&id)
                && let Object::Dictionary(d) = &mut obj
            {
                d.set("Parent", pages_id);
            }
            out.objects.insert(id, obj);
        }
        out.max_id = out.max_id.max(doc.max_id);
        if bookmarks && let Some(&first) = pages.first() {
            out.add_bookmark(Bookmark::new(title, [0.0, 0.0, 0.0], 0, first), None);
        }
        kids.extend(pages.into_iter().map(Object::Reference));
    }
    let count = kids.len() as i64;
    out.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        }),
    );
    let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
    if let Some(outline) = out.build_outline() {
        catalog.set("Outlines", outline);
        catalog.set("PageMode", "UseOutlines");
    }
    let catalog_id = out.add_object(catalog);
    out.trailer.set("Root", catalog_id);
    out.prune_objects();
    out
}

impl Tool for PdfMerge {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.merge".into(),
            name: "PDF 병합".into(),
            category: "pdf".into(),
            description: "여러 PDF를 넣은 순서대로 하나로 합칩니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Combine
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<MergeOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: MergeOptions = parse_options(opts)?;
        if input.len() < 2 {
            return Err(ToolError::Failed(
                "합칠 PDF를 두 개 이상 넣어 주세요.".into(),
            ));
        }
        let mut docs = Vec::with_capacity(input.len());
        for (i, p) in input.iter().enumerate() {
            ctx.check_cancel()?;
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            let doc = open(p).map_err(|e| ToolError::Failed(format!("{name}: {e}")))?;
            docs.push((stem(p), doc));
            ctx.progress((i + 1) as f32 / (input.len() + 1) as f32);
        }
        let mut doc = merge(docs, o.bookmarks);
        let rel = PathBuf::from(format!("{}-merged.pdf", stem(&input[0])));
        save(&mut doc, &out_path(out_dir, &rel)?)?;
        ctx.progress(1.0);
        Ok(ToolOutput::Files(vec![rel]))
    }
}
