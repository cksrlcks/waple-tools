//! 파일명 일괄 변경 (추가 설치, Rename): 규칙을 조합해 입력 순서대로 새 이름만 정한다.
//! 파일은 건드리지 않는다 (수정한 날짜만 읽는다). 충돌 확인·이름 바꾸기·되돌리기는 배치 엔진(`batch::rename`)이 한다.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use regex::{NoExpand, Regex, RegexBuilder};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum CaseMode {
    /// 소문자
    #[default]
    Lower,
    /// 대문자
    Upper,
    /// 단어 첫 글자만 대문자
    Title,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Position {
    /// 이름 앞
    Before,
    /// 이름 뒤
    #[default]
    After,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum DateSource {
    /// 오늘
    #[default]
    Today,
    /// 파일을 수정한 날짜
    Modified,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum DateFormat {
    /// 2026-11-12
    #[default]
    Dash,
    /// 20261112
    Compact,
    /// 261112
    Short,
    /// 20261112-153045 (시각 포함)
    WithTime,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum ExtCase {
    /// 그대로
    #[default]
    Keep,
    /// 소문자
    Lower,
    /// 대문자
    Upper,
}

/// 규칙은 아래 순서대로 확장자를 뺀 이름에 적용한다
#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct RenameOptions {
    /// 찾아 바꾸기
    #[schemars(extend("x-group" = "찾아 바꾸기", "x-group-toggle" = true))]
    replace: bool,
    /// 찾을 글자
    #[schemars(extend("x-group" = "찾아 바꾸기", "x-wide" = true, "x-enabled-when" = { "replace": [true] }))]
    find: String,
    /// 바꿀 글자
    ///
    /// 비우면 찾은 글자를 지웁니다. 정규식이면 $1처럼 괄호로 묶은 부분을 쓸 수 있습니다.
    #[schemars(extend("x-group" = "찾아 바꾸기", "x-wide" = true, "x-enabled-when" = { "replace": [true] }))]
    replace_with: String,
    /// 정규식
    #[schemars(extend("x-group" = "찾아 바꾸기", "x-enabled-when" = { "replace": [true] }))]
    regex: bool,
    /// 대소문자 구분
    #[schemars(extend("x-group" = "찾아 바꾸기", "x-enabled-when" = { "replace": [true] }))]
    match_case: bool,

    /// 대소문자 바꾸기
    #[schemars(extend("x-group" = "대소문자", "x-group-toggle" = true))]
    change_case: bool,
    /// 모양
    #[schemars(extend("x-group" = "대소문자", "x-enabled-when" = { "changeCase": [true] }))]
    case_mode: CaseMode,

    /// 공백을 하이픈(-)으로
    ///
    /// 이어진 공백은 하이픈 하나로 바꾸고, 앞뒤 공백은 지웁니다.
    #[schemars(extend("x-group" = "공백"))]
    spaces_to_hyphen: bool,

    /// 앞뒤에 붙이기
    #[schemars(extend("x-group" = "앞뒤에 붙이기", "x-group-toggle" = true))]
    affix: bool,
    /// 앞에 붙일 글자
    #[schemars(extend("x-group" = "앞뒤에 붙이기", "x-enabled-when" = { "affix": [true] }))]
    prefix: String,
    /// 뒤에 붙일 글자
    #[schemars(extend("x-group" = "앞뒤에 붙이기", "x-enabled-when" = { "affix": [true] }))]
    suffix: String,

    /// 날짜 붙이기
    #[schemars(extend("x-group" = "날짜", "x-group-toggle" = true))]
    date: bool,
    /// 날짜
    #[schemars(extend("x-group" = "날짜", "x-enabled-when" = { "date": [true] }))]
    date_source: DateSource,
    /// 형식
    #[schemars(extend("x-group" = "날짜", "x-enabled-when" = { "date": [true] }))]
    date_format: DateFormat,
    /// 위치
    #[schemars(extend("x-group" = "날짜", "x-enabled-when" = { "date": [true] }))]
    date_position: Position,
    /// 구분 글자
    #[schemars(extend("x-group" = "날짜", "x-enabled-when" = { "date": [true] }))]
    date_separator: String,

    /// 번호 붙이기
    #[schemars(extend("x-group" = "번호", "x-group-toggle" = true, "x-group-note" = "목록 순서대로 붙입니다."))]
    number: bool,
    /// 시작 번호
    #[schemars(range(min = 0))]
    #[schemars(extend("x-group" = "번호", "x-enabled-when" = { "number": [true] }))]
    start: u32,
    /// 자릿수
    ///
    /// 모자라면 앞을 0으로 채웁니다 (3자리: 001).
    #[schemars(range(min = 1, max = 8))]
    #[schemars(extend("x-group" = "번호", "x-enabled-when" = { "number": [true] }))]
    digits: u32,
    /// 위치
    #[schemars(extend("x-group" = "번호", "x-enabled-when" = { "number": [true] }))]
    number_position: Position,
    /// 구분 글자
    #[schemars(extend("x-group" = "번호", "x-enabled-when" = { "number": [true] }))]
    number_separator: String,

    /// 확장자
    #[schemars(extend("x-group" = "확장자"))]
    ext_case: ExtCase,
}

impl Default for RenameOptions {
    fn default() -> Self {
        Self {
            replace: false,
            find: String::new(),
            replace_with: String::new(),
            regex: false,
            match_case: true,
            change_case: false,
            case_mode: CaseMode::Lower,
            spaces_to_hyphen: false,
            affix: false,
            prefix: String::new(),
            suffix: String::new(),
            date: false,
            date_source: DateSource::Today,
            date_format: DateFormat::Dash,
            date_position: Position::Before,
            date_separator: "_".into(),
            number: false,
            start: 1,
            digits: 3,
            number_position: Position::After,
            number_separator: "_".into(),
            ext_case: ExtCase::Keep,
        }
    }
}

struct MiscRename;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscRename)
}

waple_tools_addon_abi::export_tool!(tool);

/// 찾아 바꾸기 정규식 (켜져 있고 찾을 글자가 있을 때만)
fn finder(o: &RenameOptions) -> Result<Option<Regex>, ToolError> {
    if !o.replace || o.find.is_empty() {
        return Ok(None);
    }
    let pattern = if o.regex {
        o.find.clone()
    } else {
        regex::escape(&o.find)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!o.match_case)
        .build()
        .map(Some)
        .map_err(|_| ToolError::InvalidOptions("찾을 정규식이 올바르지 않습니다.".into()))
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if start {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        start = !c.is_alphanumeric();
    }
    out
}

/// 이름이 비었으면 붙일 것만, 아니면 구분 글자를 끼워 앞이나 뒤에
fn attach(stem: String, add: &str, sep: &str, pos: Position) -> String {
    match pos {
        _ if stem.is_empty() => add.to_string(),
        Position::Before => format!("{add}{sep}{stem}"),
        Position::After => format!("{stem}{sep}{add}"),
    }
}

fn date_text(t: DateTime<Local>, f: DateFormat) -> String {
    let fmt = match f {
        DateFormat::Dash => "%Y-%m-%d",
        DateFormat::Compact => "%Y%m%d",
        DateFormat::Short => "%y%m%d",
        DateFormat::WithTime => "%Y%m%d-%H%M%S",
    };
    t.format(fmt).to_string()
}

/// `index`: 입력 목록에서 몇 번째인지 (번호 붙이기)
fn new_name(
    path: &Path,
    index: usize,
    o: &RenameOptions,
    find: Option<&Regex>,
    now: DateTime<Local>,
) -> Result<String, ToolError> {
    // `.gitignore`처럼 점으로 시작하는 이름은 통째로 이름 (확장자 없음)
    let mut stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());

    if let Some(re) = find {
        stem = if o.regex {
            re.replace_all(&stem, o.replace_with.as_str()).into_owned()
        } else {
            re.replace_all(&stem, NoExpand(&o.replace_with))
                .into_owned()
        };
    }
    if o.change_case {
        stem = match o.case_mode {
            CaseMode::Lower => stem.to_lowercase(),
            CaseMode::Upper => stem.to_uppercase(),
            CaseMode::Title => title_case(&stem),
        };
    }
    if o.spaces_to_hyphen {
        stem = stem.split_whitespace().collect::<Vec<_>>().join("-");
    }
    if o.affix {
        stem = format!("{}{stem}{}", o.prefix, o.suffix);
    }
    if o.date {
        let t = match o.date_source {
            DateSource::Today => now,
            DateSource::Modified => DateTime::from(std::fs::metadata(path)?.modified()?),
        };
        stem = attach(
            stem,
            &date_text(t, o.date_format),
            &o.date_separator,
            o.date_position,
        );
    }
    if o.number {
        let n = u64::from(o.start) + index as u64;
        let width = o.digits.clamp(1, 8) as usize;
        stem = attach(
            stem,
            &format!("{n:0width$}"),
            &o.number_separator,
            o.number_position,
        );
    }
    // 이름이 비면 빈 이름을 돌려준다: 엔진이 "새 이름이 비어 있음"으로 막는다 (`.txt`만 남기지 않는다)
    if stem.is_empty() {
        return Ok(String::new());
    }
    Ok(match ext {
        None => stem,
        Some(ext) => {
            let ext = match o.ext_case {
                ExtCase::Keep => ext,
                ExtCase::Lower => ext.to_lowercase(),
                ExtCase::Upper => ext.to_uppercase(),
            };
            format!("{stem}.{ext}")
        }
    })
}

impl Tool for MiscRename {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.rename".into(),
            name: "파일명 일괄 변경".into(),
            category: "misc".into(),
            description: "규칙을 조합해 여러 파일 이름을 한 번에 바꿉니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Rename
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(RenameOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: RenameOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        ctx.check_cancel()?;
        let find = finder(&o)?;
        let now = Local::now();
        input
            .iter()
            .enumerate()
            .map(|(i, p)| new_name(p, i, &o, find.as_ref(), now).map(PathBuf::from))
            .collect::<Result<_, _>>()
            .map(ToolOutput::Files)
    }
}

#[cfg(test)]
mod tests;
