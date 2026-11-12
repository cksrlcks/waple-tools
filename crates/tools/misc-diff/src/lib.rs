//! 텍스트 비교 (추가 설치, Interactive, 입력 파일 없음): 두 글의 다른 줄과 줄 안의 바뀐 글자

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::Deserialize;
use similar::{Algorithm, DiffTag as Op, capture_diff_slices_deadline};
use waple_tools_core::{
    Block, DiffLayout, DiffLine, DiffTag, JobContext, Span, Tool, ToolError, ToolKind, ToolMeta,
    ToolOutput,
};

/// 편집할 때마다 다시 돌므로 시간을 묶는다. 넘으면 similar가 덜 촘촘하지만 맞는 결과를 낸다
const LINE_TIMEOUT: Duration = Duration::from_millis(300);
const INLINE_TIMEOUT: Duration = Duration::from_millis(200);
/// 한 줄 쌍의 글자 비교 상한 (넘으면 줄 전체만 표시)
const INLINE_MAX_CHARS: usize = 20_000;
/// DLL 경계로 넘기는 JSON과 앱 화면이 감당할 만큼만 보낸다
const MAX_LINES: usize = 20_000;
const MAX_BYTES: usize = 2_000_000;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Layout {
    /// 좌우
    #[default]
    Split,
    /// 통합
    Unified,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none", "x-layout" = "full"))]
struct DiffOptions {
    /// 원본
    ///
    /// 비교의 기준이 되는 글입니다. 붙여 넣거나 파일을 놓으세요.
    #[schemars(extend("x-multiline" = true))]
    original: String,
    /// 비교할 글
    ///
    /// 원본과 견줄 글입니다. 붙여 넣거나 파일을 놓으세요.
    #[schemars(extend("x-multiline" = true))]
    modified: String,
    /// 표시
    ///
    /// 좌우는 두 글을 나란히, 통합은 지운 줄과 넣은 줄을 한 열에 차례로 보여 줍니다.
    layout: Layout,
    /// 공백 무시
    ///
    /// 띄어쓰기·탭이 달라도 같은 줄로 봅니다. 화면에는 원래 글을 그대로 보여 줍니다.
    ignore_whitespace: bool,
    /// 대소문자 무시
    ///
    /// 영문 대소문자가 달라도 같은 줄로 봅니다. 화면에는 원래 글을 그대로 보여 줍니다.
    ignore_case: bool,
}

struct MiscDiff;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscDiff)
}

waple_tools_addon_abi::export_tool!(tool);

/// 비교할 때만 쓰는 정규화 (표시는 늘 원문)
#[derive(Clone, Copy)]
struct Norm {
    ws: bool,
    case: bool,
}

impl Norm {
    fn key(self, c: char) -> Option<char> {
        if self.ws && c.is_whitespace() {
            return None;
        }
        Some(if self.case {
            c.to_lowercase().next().unwrap_or(c)
        } else {
            c
        })
    }

    fn line(self, s: &str) -> Cow<'_, str> {
        if self.ws || self.case {
            Cow::Owned(s.chars().filter_map(|c| self.key(c)).collect())
        } else {
            Cow::Borrowed(s)
        }
    }

    /// 비교에 쓰는 글자와 그 글자의 원문 위치 (글자 순번)
    fn keys(self, s: &str) -> (Vec<char>, Vec<usize>) {
        s.chars()
            .enumerate()
            .filter_map(|(i, c)| Some((self.key(c)?, i)))
            .unzip()
    }
}

fn plain(text: &str) -> Vec<Span> {
    vec![Span {
        text: text.into(),
        mark: None,
    }]
}

fn spans(text: &str, marked: &[bool]) -> Vec<Span> {
    let mut out: Vec<Span> = vec![];
    for (c, &m) in text.chars().zip(marked) {
        let mark = m.then_some(0);
        match out.last_mut() {
            Some(s) if s.mark == mark => s.text.push(c),
            _ => out.push(Span {
                text: c.to_string(),
                mark,
            }),
        }
    }
    if out.is_empty() { plain(text) } else { out }
}

/// 짝지은 두 줄에서 바뀐 글자 표시. 너무 다르거나 길면 None (줄 전체가 바뀐 것으로 본다)
fn inline(a: &str, b: &str, norm: Norm, deadline: Instant) -> Option<(Vec<Span>, Vec<Span>)> {
    if a.len() + b.len() > INLINE_MAX_CHARS * 4 || Instant::now() >= deadline {
        return None;
    }
    let (ca, pa) = norm.keys(a);
    let (cb, pb) = norm.keys(b);
    let total = ca.len() + cb.len();
    if total == 0 || total > INLINE_MAX_CHARS {
        return None;
    }
    let ops = capture_diff_slices_deadline(Algorithm::Myers, &ca, &cb, Some(deadline));
    let same: usize = ops
        .iter()
        .filter(|op| op.tag() == Op::Equal)
        .map(|op| op.old_range().len())
        .sum();
    // ponytail: 같은 글자가 절반 미만(similar ratio < 0.5)이면 표시가 소음이라 생략. 단어 단위 비교가 필요하면 그때 바꾼다
    if same * 4 < total {
        return None;
    }
    let mut ma = vec![false; a.chars().count()];
    let mut mb = vec![false; b.chars().count()];
    for op in ops.iter().filter(|op| op.tag() != Op::Equal) {
        op.old_range().for_each(|i| ma[pa[i]] = true);
        op.new_range().for_each(|j| mb[pb[j]] = true);
    }
    Some((spans(a, &ma), spans(b, &mb)))
}

/// 보낼 줄 모음 (상한을 넘으면 멈춘다)
#[derive(Default)]
struct Out {
    lines: Vec<DiffLine>,
    bytes: usize,
    full: bool,
}

impl Out {
    fn push(&mut self, tag: DiffTag, old: Option<usize>, new: Option<usize>, spans: Vec<Span>) {
        let bytes: usize = spans.iter().map(|s| s.text.len()).sum();
        if self.full || self.lines.len() >= MAX_LINES || self.bytes + bytes > MAX_BYTES {
            self.full = true;
            return;
        }
        self.bytes += bytes;
        let num = |n: usize| (n + 1) as u32;
        self.lines.push(DiffLine {
            tag,
            old: old.map(num),
            new: new.map(num),
            spans,
        });
    }
}

fn build(o: &DiffOptions) -> Vec<Block> {
    if o.original.is_empty() && o.modified.is_empty() {
        return vec![Block::Text {
            text: "원본과 비교할 글을 붙여 넣거나 파일 두 개를 놓아 주세요.".into(),
        }];
    }
    let norm = Norm {
        ws: o.ignore_whitespace,
        case: o.ignore_case,
    };
    // lines(): \n과 \r\n 둘 다 줄 끝으로 본다
    let old: Vec<&str> = o.original.lines().collect();
    let new: Vec<&str> = o.modified.lines().collect();
    let ko: Vec<_> = old.iter().map(|l| norm.line(l)).collect();
    let kn: Vec<_> = new.iter().map(|l| norm.line(l)).collect();
    let ops = capture_diff_slices_deadline(
        Algorithm::Patience,
        &ko,
        &kn,
        Some(Instant::now() + LINE_TIMEOUT),
    );

    let (mut deleted, mut inserted) = (0, 0);
    for op in ops.iter().filter(|op| op.tag() != Op::Equal) {
        deleted += op.old_range().len();
        inserted += op.new_range().len();
    }
    if deleted + inserted == 0 {
        let text = if old == new {
            "두 글이 같습니다."
        } else {
            "무시한 차이 말고는 두 글이 같습니다."
        };
        return vec![Block::Text { text: text.into() }];
    }

    let deadline = Instant::now() + INLINE_TIMEOUT;
    let mut out = Out::default();
    let mut i = 0;
    while i < ops.len() && !out.full {
        let (tag, mut or, mut nr) = ops[i].as_tag_tuple();
        i += 1;
        if tag == Op::Equal {
            for (a, b) in or.zip(nr) {
                out.push(DiffTag::Equal, Some(a), Some(b), plain(old[a]));
            }
            continue;
        }
        // 이어진 삭제·추가·교체를 한 묶음으로: 지운 줄 다음에 넣은 줄
        while i < ops.len() && ops[i].tag() != Op::Equal {
            let (_, o2, n2) = ops[i].as_tag_tuple();
            or.end = o2.end;
            nr.end = n2.end;
            i += 1;
        }
        let mut dels: Vec<Vec<Span>> = or.clone().map(|a| plain(old[a])).collect();
        let mut inss: Vec<Vec<Span>> = nr.clone().map(|b| plain(new[b])).collect();
        // 좌우 표시와 같이 k번째 지운 줄 ↔ k번째 넣은 줄을 짝지어 글자 비교
        for k in 0..or.len().min(nr.len()) {
            if let Some((x, y)) = inline(old[or.start + k], new[nr.start + k], norm, deadline) {
                dels[k] = x;
                inss[k] = y;
            }
        }
        for (a, s) in or.zip(dels) {
            out.push(DiffTag::Delete, Some(a), None, s);
        }
        for (b, s) in nr.zip(inss) {
            out.push(DiffTag::Insert, None, Some(b), s);
        }
    }

    let mut blocks = vec![];
    if out.full {
        blocks.push(Block::Text {
            text: format!(
                "결과가 너무 길어 앞부분 {}줄만 보여 줍니다. 줄 수 집계는 전체 기준입니다.",
                out.lines.len()
            ),
        });
    }
    let count = |label: &str, n: usize| Block::Number {
        label: label.into(),
        value: n as f64,
        unit: Some("줄".into()),
    };
    blocks.push(count("추가된 줄", inserted));
    blocks.push(count("삭제된 줄", deleted));
    blocks.push(Block::Diff {
        layout: match o.layout {
            Layout::Split => DiffLayout::Split,
            Layout::Unified => DiffLayout::Unified,
        },
        lines: out.lines,
    });
    blocks
}

impl Tool for MiscDiff {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.diff".into(),
            name: "텍스트 비교".into(),
            category: "misc".into(),
            description: "두 텍스트의 다른 곳을 줄·글자 단위로 보여 줍니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(DiffOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: DiffOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        ctx.check_cancel()?;
        Ok(ToolOutput::Blocks(build(&opts)))
    }
}

#[cfg(test)]
mod tests;
