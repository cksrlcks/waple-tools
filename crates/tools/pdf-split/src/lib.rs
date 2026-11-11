//! PDF 분할 / 페이지 추출 (추가 설치, Generator): 페이지마다 · 범위 지정 · N페이지 단위 → `<이름>-split/` 폴더

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_pdf_common::{
    PDF_EXTS, drop_page_refs, open, out_path, pages_in, parse_options, parse_ranges, save,
    schema_of, stem,
};
use waple_tools_pdf_render as render;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 페이지마다
    #[default]
    Each,
    /// 범위 지정
    Ranges,
    /// N페이지 단위
    Chunk,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-single-input" = true,
    // 작업대에 쪽 썸네일을 보여 주고, "범위 지정"일 때 고른 쪽을 `ranges`에 적는다
    "x-page-picker" = "ranges",
    "x-preview" = "none"
))]
struct SplitOptions {
    /// 나누는 방식
    mode: Mode,
    /// 페이지 범위
    ///
    /// 작업대의 썸네일을 눌러 고릅니다. 이어진 쪽끼리 파일 하나가 됩니다.
    #[schemars(extend("x-enabled-when" = { "mode": ["ranges"] }))]
    ranges: String,
    /// 범위를 하나의 파일로 합치기
    ///
    /// 끄면 범위마다 파일을 하나씩 만듭니다.
    #[schemars(extend("x-enabled-when" = { "mode": ["ranges"] }))]
    join: bool,
    /// 몇 페이지씩
    #[schemars(range(min = 1))]
    #[schemars(extend("x-enabled-when" = { "mode": ["chunk"] }))]
    chunk: u32,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Each,
            ranges: String::new(),
            join: false,
            chunk: 2,
        }
    }
}

struct PdfSplit;

pub fn tool() -> Box<dyn Tool> {
    Box::new(PdfSplit)
}

waple_tools_addon_abi::export_tool!(tool);

/// `(1,3)` → `1-3`, `(5,5)` → `5`
fn label(&(a, b): &(u32, u32)) -> String {
    if a == b {
        a.to_string()
    } else {
        format!("{a}-{b}")
    }
}

/// 만들 파일들: (이름 꼬리 `p1-3`, 남길 페이지)
fn groups(o: &SplitOptions, pages: u32) -> Result<Vec<(String, Vec<u32>)>, ToolError> {
    let range = |r: (u32, u32)| (format!("p{}", label(&r)), (r.0..=r.1).collect());
    Ok(match o.mode {
        Mode::Each => (1..=pages).map(|p| range((p, p))).collect(),
        Mode::Chunk => {
            let n = o.chunk.max(1);
            (0..pages.div_ceil(n))
                .map(|i| range((i * n + 1, ((i + 1) * n).min(pages))))
                .collect()
        }
        // 빈 칸 = 전체를 한 파일로 (썸네일을 모두 고른 상태)
        Mode::Ranges => {
            let rs = parse_ranges(&o.ranges, pages)?;
            if o.join {
                let name = rs.iter().map(label).collect::<Vec<_>>().join("_");
                vec![(format!("p{name}"), pages_in(&rs))]
            } else {
                rs.into_iter().map(range).collect()
            }
        }
    })
}

impl Tool for PdfSplit {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf.split".into(),
            name: "PDF 분할".into(),
            category: "pdf".into(),
            description: "페이지마다, 범위별로 PDF를 나누거나 페이지를 뽑아냅니다.".into(),
            extensions: PDF_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Generator
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<SplitOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: SplitOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("PDF를 넣어 주세요.".into()))?;
        if render::wants_thumbnails(opts) {
            return render::thumbnails(ctx, &render::open_file(src)?, out_dir);
        }
        let name = stem(src);
        let mut doc = open(src)?;
        drop_page_refs(&mut doc);
        let pages = doc.get_pages().len() as u32;
        let groups = groups(&o, pages)?;
        let top = PathBuf::from(format!("{name}-split"));
        let mut names = Vec::with_capacity(groups.len());
        for (i, (tail, keep)) in groups.iter().enumerate() {
            ctx.check_cancel()?;
            // ponytail: 조각마다 문서 전체를 복제한다. 수백 쪽 대용량에서 느리면 필요한 객체만 옮기는 방식으로
            let mut part = doc.clone();
            let drop: Vec<u32> = (1..=pages).filter(|p| !keep.contains(p)).collect();
            part.delete_pages(&drop);
            part.prune_objects();
            let rel = top.join(format!("{name}_{tail}.pdf"));
            save(&mut part, &out_path(out_dir, &rel)?)?;
            names.push(rel);
            ctx.progress((i + 1) as f32 / groups.len() as f32);
        }
        Ok(ToolOutput::Files(names))
    }
}
