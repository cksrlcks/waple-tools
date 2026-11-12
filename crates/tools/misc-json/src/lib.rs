//! JSON 포맷터 (추가 설치, Interactive, 입력 파일 없음): 붙여넣은 JSON → 들여쓰기·압축·키 정렬
//!
//! 값은 바꾸지 않는다: 숫자·문자열은 원문 글자를 그대로 옮기고, 키 순서·중복 키도 그대로 둔다.
//! 그래서 serde_json으로 값을 읽지 않고 원문 조각을 들고 있는 작은 파서를 쓴다.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Indent {
    /// 2칸
    #[default]
    Two,
    /// 4칸
    Four,
    /// 탭
    Tab,
    /// 압축(한 줄)
    Minify,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none"))]
struct JsonOptions {
    /// JSON
    ///
    /// 정리할 JSON입니다. 붙여넣거나 파일을 끌어다 놓으세요.
    #[schemars(extend("x-multiline" = true))]
    text: String,
    /// 들여쓰기
    ///
    /// 압축을 고르면 공백 없이 한 줄로 만듭니다.
    indent: Indent,
    /// 키 정렬
    ///
    /// 객체의 키를 가나다·알파벳 순으로 정렬합니다. 안쪽 객체도 함께 정렬합니다.
    sort_keys: bool,
}

struct MiscJson;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscJson)
}

waple_tools_addon_abi::export_tool!(tool);

/// 이보다 깊게 중첩되면 읽지 않는다 (재귀로 읽고 쓰므로 스택을 지킨다)
const MAX_DEPTH: usize = 512;

/// 원문 조각을 그대로 들고 있는 JSON 값
enum Value<'a> {
    /// 숫자·문자열(따옴표 포함)·true·false·null 원문
    Raw(&'a str),
    Array(Vec<Value<'a>>),
    /// (키 원문(따옴표 포함), 값). 중복 키도 그대로
    Object(Vec<(&'a str, Value<'a>)>),
}

/// (바이트 위치, 오류 문구)
type ParseError = (usize, &'static str);

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn err<T>(&self, msg: &'static str) -> Result<T, ParseError> {
        Err((self.i, msg))
    }

    fn value(&mut self, depth: usize) -> Result<Value<'a>, ParseError> {
        self.ws();
        match self.peek() {
            None => self.err("값이 와야 하는데 JSON이 끝났습니다."),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::Raw),
            Some(b'-' | b'0'..=b'9') => self.number().map(Value::Raw),
            Some(b'\'') => self.err("문자열은 큰따옴표(\")로 감싸야 합니다."),
            Some(_) => {
                for word in ["true", "false", "null"] {
                    if self.s[self.i..].starts_with(word) {
                        let start = self.i;
                        self.i += word.len();
                        return Ok(Value::Raw(&self.s[start..self.i]));
                    }
                }
                self.err("여기에 값(문자열·숫자·객체·배열·true·false·null)이 와야 합니다.")
            }
        }
    }

    fn enter(&self, depth: usize) -> Result<(), ParseError> {
        if depth >= MAX_DEPTH {
            return self.err("중첩이 너무 깊습니다. 512단계까지만 읽을 수 있습니다.");
        }
        Ok(())
    }

    fn array(&mut self, depth: usize) -> Result<Value<'a>, ParseError> {
        self.enter(depth)?;
        self.i += 1;
        let mut items = vec![];
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    let comma = self.i;
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some(b']') {
                        return Err((comma, "마지막 쉼표 뒤에 값이 없습니다. 쉼표를 지워 주세요."));
                    }
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::Array(items));
                }
                None => return self.err("배열이 닫히지 않았습니다. ]가 필요합니다."),
                Some(_) => return self.err("쉼표(,)나 닫는 괄호(])가 필요합니다."),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value<'a>, ParseError> {
        self.enter(depth)?;
        self.i += 1;
        let mut members = vec![];
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.ws();
            let key = match self.peek() {
                Some(b'"') => self.string()?,
                Some(b'\'') => return self.err("키는 큰따옴표(\")로 감싸야 합니다."),
                None => return self.err("객체가 닫히지 않았습니다. }가 필요합니다."),
                Some(_) => return self.err("큰따옴표로 감싼 키가 필요합니다."),
            };
            self.ws();
            if self.peek() != Some(b':') {
                return self.err("키 뒤에 콜론(:)이 필요합니다.");
            }
            self.i += 1;
            members.push((key, self.value(depth + 1)?));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    let comma = self.i;
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some(b'}') {
                        return Err((comma, "마지막 쉼표 뒤에 값이 없습니다. 쉼표를 지워 주세요."));
                    }
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Object(members));
                }
                None => return self.err("객체가 닫히지 않았습니다. }가 필요합니다."),
                Some(_) => return self.err("쉼표(,)나 닫는 괄호(})가 필요합니다."),
            }
        }
    }

    /// 따옴표까지 포함한 원문. 이스케이프는 검사만 하고 풀지 않는다
    fn string(&mut self) -> Result<&'a str, ParseError> {
        let start = self.i;
        self.i += 1;
        loop {
            match self.peek() {
                None => {
                    return Err((start, "문자열이 닫히지 않았습니다. 끝에 \"가 필요합니다."));
                }
                Some(b'"') => {
                    self.i += 1;
                    return Ok(&self.s[start..self.i]);
                }
                Some(b'\\') => {
                    self.i += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.i += 1,
                        Some(b'u') => {
                            self.i += 1;
                            for _ in 0..4 {
                                if !self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                    return self.err("\\u 뒤에는 16진수 4자리가 와야 합니다.");
                                }
                                self.i += 1;
                            }
                        }
                        _ => return self.err("잘못된 이스케이프입니다. \\ 뒤에는 \" \\ / b f n r t u만 올 수 있습니다."),
                    }
                }
                Some(c) if c < 0x20 => {
                    return self.err(
                        "문자열 안에는 줄바꿈·탭을 그대로 쓸 수 없습니다. \\n·\\t로 적어 주세요.",
                    );
                }
                // UTF-8 이어지는 바이트도 ASCII가 아니므로 그냥 넘긴다
                Some(_) => self.i += 1,
            }
        }
    }

    fn number(&mut self) -> Result<&'a str, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return self.err("숫자는 0으로 시작할 수 없습니다.");
                }
            }
            Some(b'1'..=b'9') => self.digits(),
            _ => return self.err("숫자 형식이 잘못되었습니다. - 뒤에 숫자가 와야 합니다."),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return self.err("숫자 형식이 잘못되었습니다. 소수점 뒤에 숫자가 와야 합니다.");
            }
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return self.err("숫자 형식이 잘못되었습니다. 지수(e) 뒤에 숫자가 와야 합니다.");
            }
            self.digits();
        }
        Ok(&self.s[start..self.i])
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
    }
}

fn parse(s: &str) -> Result<Value<'_>, ParseError> {
    let mut p = Parser { s, i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i < s.len() {
        return p.err("JSON 값 뒤에 다른 내용이 있습니다. 값은 하나만 둘 수 있습니다.");
    }
    Ok(v)
}

/// 바이트 위치 → "N번째 줄 M번째 글자"
fn position(s: &str, at: usize) -> String {
    let before = &s[..at.min(s.len())];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    format!("{line}번째 줄 {col}번째 글자")
}

/// indent가 None이면 압축
fn write(v: &Value, indent: Option<&str>, sort: bool, level: usize, out: &mut String) {
    let newline = |out: &mut String, level: usize| {
        if let Some(ind) = indent {
            out.push('\n');
            for _ in 0..level {
                out.push_str(ind);
            }
        }
    };
    match v {
        Value::Raw(s) => out.push_str(s),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(members) if members.is_empty() => out.push_str("{}"),
        Value::Array(items) => {
            out.push('[');
            for (n, item) in items.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                write(item, indent, sort, level + 1, out);
            }
            newline(out, level);
            out.push(']');
        }
        Value::Object(members) => {
            let mut members: Vec<_> = members.iter().collect();
            if sort {
                // 이스케이프를 푼 글자로 비교한다. 같은 키(중복)는 원래 순서를 지킨다
                members.sort_by_cached_key(|(k, _)| {
                    serde_json::from_str::<String>(k).unwrap_or_else(|_| k.to_string())
                });
            }
            out.push('{');
            for (n, (key, value)) in members.into_iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                out.push_str(key);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write(value, indent, sort, level + 1, out);
            }
            newline(out, level);
            out.push('}');
        }
    }
}

fn build(o: &JsonOptions) -> Vec<Block> {
    let text = o.text.strip_prefix('\u{feff}').unwrap_or(&o.text);
    if text.trim().is_empty() {
        return vec![Block::Text {
            text: "JSON을 붙여넣거나 파일을 끌어다 놓으세요.".into(),
        }];
    }
    let value = match parse(text) {
        Ok(v) => v,
        Err((at, msg)) => {
            return vec![Block::Text {
                text: format!("{}: {msg}", position(text, at)),
            }];
        }
    };
    let indent = match o.indent {
        Indent::Two => Some("  "),
        Indent::Four => Some("    "),
        Indent::Tab => Some("\t"),
        Indent::Minify => None,
    };
    let mut code = String::with_capacity(text.len());
    write(&value, indent, o.sort_keys, 0, &mut code);
    let size = |label: &str, bytes: usize| Block::Number {
        label: label.into(),
        value: bytes as f64,
        unit: Some("B".into()),
    };
    let (before, after) = (
        size("원래 크기", o.text.len()),
        size("결과 크기", code.len()),
    );
    vec![
        Block::Code {
            lang: "json".into(),
            code,
            name: Some("formatted.json".into()),
        },
        before,
        after,
    ]
}

impl Tool for MiscJson {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.json".into(),
            name: "JSON 포맷터".into(),
            category: "misc".into(),
            description: "JSON을 보기 좋게 정리하거나 압축합니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(JsonOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: JsonOptions = serde_json::from_value(if opts.is_null() {
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
