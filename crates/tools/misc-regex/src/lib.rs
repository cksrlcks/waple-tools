//! 정규식 테스트 (추가 설치, Interactive, 입력 파일 없음): 일치 강조 + 일치·그룹 카드

use std::path::{Path, PathBuf};

use fancy_regex::{CompileError, Error, ParseError, RegexBuilder};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{
    Block, JobContext, Record, RecordField, Span, Tool, ToolError, ToolKind, ToolMeta, ToolOutput,
};

/// 강조·카드에 보여 줄 최대 일치 수 (개수는 전부 센다)
const MAX_SHOWN: usize = 1000;

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none", "x-layout" = "split"))]
struct RegexOptions {
    /// 모두 찾기
    ///
    /// JS의 g 플래그. 끄면 첫 번째 일치만 찾습니다.
    #[schemars(extend("x-short" = "g", "x-group" = "플래그", "default" = true))]
    global: bool,
    /// 대소문자 무시
    ///
    /// JS의 i 플래그. A와 a를 같은 글자로 봅니다.
    #[schemars(extend("x-short" = "i", "x-group" = "플래그"))]
    ignore_case: bool,
    /// 여러 줄
    ///
    /// JS의 m 플래그. ^·$가 줄마다 맞습니다. 끄면 글 전체의 처음과 끝에만 맞습니다.
    #[schemars(extend("x-short" = "m", "x-group" = "플래그"))]
    multi_line: bool,
    /// 점(.)이 줄바꿈도 포함
    ///
    /// JS의 s 플래그. 끄면 .은 줄바꿈을 건너뜁니다.
    #[schemars(extend("x-short" = "s", "x-group" = "플래그"))]
    dot_all: bool,
    /// 정규식
    ///
    /// JavaScript 정규식과 거의 같습니다. 이름 그룹 (?<name>…), 앞뒤 탐색, 역참조 \1 \k<name>을 쓸 수 있습니다. 뒤쪽 탐색 (?<=…)은 길이가 정해진 패턴만 됩니다.
    #[schemars(extend("format" = "regex"))]
    pattern: String,
    /// 테스트 문자열
    ///
    /// 정규식을 맞춰 볼 글입니다. 붙여 넣거나 파일을 끌어다 놓으세요.
    #[schemars(extend("x-multiline" = true))]
    text: String,
}

impl Default for RegexOptions {
    fn default() -> Self {
        Self {
            global: true,
            ignore_case: false,
            multi_line: false,
            dot_all: false,
            pattern: String::new(),
            text: String::new(),
        }
    }
}

struct MiscRegex;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscRegex)
}

waple_tools_addon_abi::export_tool!(tool);

fn text(s: impl Into<String>) -> Block {
    Block::Text { text: s.into() }
}

fn parse_error(e: &ParseError) -> String {
    match e {
        ParseError::UnclosedOpenParen => "여는 괄호 (에 맞는 닫는 괄호 )가 없습니다.".into(),
        ParseError::InvalidRepeat => "반복 기호(*, +, ?, {n})를 쓸 수 없는 자리입니다.".into(),
        ParseError::RecursionExceeded => "괄호가 너무 깊게 겹쳐 있습니다.".into(),
        ParseError::TrailingBackslash => "맨 끝의 \\ 뒤에 글자가 없습니다.".into(),
        ParseError::InvalidEscape(s) => format!("알 수 없는 이스케이프입니다: {s}"),
        ParseError::UnclosedUnicodeName => "유니코드 이스케이프가 닫히지 않았습니다.".into(),
        ParseError::InvalidHex => "16진수 이스케이프가 올바르지 않습니다.".into(),
        ParseError::InvalidCodepointValue => "없는 유니코드 글자 번호입니다.".into(),
        ParseError::InvalidClass => "문자 집합 […]이 올바르지 않습니다.".into(),
        ParseError::UnknownFlag(s) => format!("알 수 없는 플래그입니다: {s}"),
        ParseError::InvalidBackref => "역참조가 올바르지 않습니다.".into(),
        ParseError::TargetNotRepeatable => "반복할 수 없는 것에 반복 기호를 붙였습니다.".into(),
        ParseError::InvalidGroupName => "그룹 이름이 올바르지 않습니다.".into(),
        ParseError::InvalidGroupNameBackref(s) => format!("없는 그룹 이름을 참조했습니다: {s}"),
        other => other.to_string(),
    }
}

fn compile_error(e: &CompileError) -> String {
    match e {
        CompileError::LookBehindNotConst => {
            "뒤쪽 탐색 (?<=…)·(?<!…) 안은 길이가 정해진 패턴이어야 합니다.".into()
        }
        CompileError::InvalidBackref(g) => format!("없는 그룹 {g}을 역참조했습니다."),
        CompileError::NamedBackrefOnly => {
            "이름 그룹을 쓰면 번호 역참조 대신 \\k<name>을 써 주세요.".into()
        }
        CompileError::InvalidGroupName => "그룹 이름이 올바르지 않습니다.".into(),
        CompileError::InvalidGroupNameBackref(s) => format!("없는 그룹 이름을 참조했습니다: {s}"),
        CompileError::FeatureNotYetSupported(s) => format!("아직 지원하지 않는 기능입니다: {s}"),
        other => other.to_string(),
    }
}

fn error_message(pattern: &str, e: &Error) -> String {
    match e {
        Error::ParseError(pos, kind) => {
            // pos는 바이트 위치: 글자 수로 바꾼다 (1부터)
            let at = pattern.get(..*pos).map_or(0, |s| s.chars().count()) + 1;
            format!(
                "정규식이 올바르지 않습니다 ({at}번째 글자): {}",
                parse_error(kind)
            )
        }
        Error::CompileError(c) => format!("정규식이 올바르지 않습니다: {}", compile_error(c)),
        Error::RuntimeError(_) => "패턴이 너무 복잡해서 찾기를 멈췄습니다. (a+)+처럼 반복 안에 반복이 겹치는 부분을 줄여 주세요.".into(),
        other => format!("정규식을 쓸 수 없습니다: {other}"),
    }
}

fn build(ctx: &JobContext, o: &RegexOptions) -> Result<Vec<Block>, ToolError> {
    if o.pattern.is_empty() {
        return Ok(vec![text("정규식을 입력해 주세요.")]);
    }
    let re = match RegexBuilder::new(&o.pattern)
        .case_insensitive(o.ignore_case)
        .multi_line(o.multi_line)
        .dot_matches_new_line(o.dot_all)
        .build()
    {
        Ok(re) => re,
        Err(e) => return Ok(vec![text(error_message(&o.pattern, &e))]),
    };
    let t = o.text.as_str();
    if t.is_empty() {
        return Ok(vec![text(
            "테스트할 글을 입력하거나 파일을 끌어다 놓아 주세요.",
        )]);
    }

    let names: Vec<String> = re
        .capture_names()
        .enumerate()
        .skip(1)
        .map(|(i, n)| n.map_or_else(|| format!("그룹 {i}"), str::to_string))
        .collect();

    let mut total = 0usize;
    let mut empty = 0usize;
    let mut spans = vec![];
    let mut items = vec![];
    let mut shown_end = 0; // 강조에 넣은 마지막 바이트
    // 글자 위치 커서 (일치는 앞에서부터 순서대로 오므로 한 번만 훑는다)
    let (mut cur, mut chars) = (0, 0);

    for caps in re.captures_iter(t) {
        ctx.check_cancel()?;
        let caps = match caps {
            Ok(c) => c,
            Err(e) => return Ok(vec![text(error_message(&o.pattern, &e))]),
        };
        let Some(m) = caps.get(0) else { continue };
        if m.start() == m.end() {
            empty += 1;
        }
        if total < MAX_SHOWN {
            chars += t[cur..m.start()].chars().count();
            cur = m.start();
            let len = m.as_str().chars().count();
            if shown_end < m.start() {
                spans.push(Span {
                    text: t[shown_end..m.start()].into(),
                    mark: None,
                });
            }
            spans.push(Span {
                text: m.as_str().into(),
                mark: Some(total as u32),
            });
            shown_end = m.end();
            items.push(Record {
                title: format!("매치 {}", total + 1),
                meta: Some(format!("위치 {chars}–{}", chars + len)),
                text: m.as_str().into(),
                fields: names
                    .iter()
                    .enumerate()
                    .map(|(i, label)| RecordField {
                        label: label.clone(),
                        value: caps.get(i + 1).map_or("", |g| g.as_str()).into(),
                    })
                    .collect(),
            });
        }
        total += 1;
        if !o.global {
            break;
        }
    }

    let mut blocks = vec![Block::Number {
        label: "일치 수".into(),
        value: total as f64,
        unit: Some("개".into()),
    }];
    if total == 0 {
        blocks.push(text("일치하는 곳이 없습니다."));
        return Ok(blocks);
    }
    if total > MAX_SHOWN {
        blocks.push(text(format!(
            "일치가 {total}개라 앞의 {MAX_SHOWN}개만 보여 줍니다."
        )));
    }
    if empty > 0 {
        blocks.push(text(format!(
            "길이가 0인 일치가 {empty}개 있습니다 (^, \\b, a* 같은 패턴). 위치만 표시됩니다."
        )));
    }
    if shown_end < t.len() {
        spans.push(Span {
            text: t[shown_end..].into(),
            mark: None,
        });
    }
    blocks.push(Block::Highlight { spans });
    blocks.push(Block::Records { items });
    Ok(blocks)
}

impl Tool for MiscRegex {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.regex".into(),
            name: "정규식 테스트".into(),
            category: "misc".into(),
            description: "정규식이 글의 어디에 맞는지와 그룹 값을 보여 줍니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(RegexOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: RegexOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        ctx.check_cancel()?;
        build(ctx, &opts).map(ToolOutput::Blocks)
    }
}

#[cfg(test)]
mod tests;
