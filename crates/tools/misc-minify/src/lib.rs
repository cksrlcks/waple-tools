//! CSS · JS · HTML 압축 (추가 설치, PerFile): 공백·주석을 줄인 같은 이름의 파일을 만든다.
//! CSS는 lightningcss, JS는 oxc, HTML은 minify-html(안의 CSS·JS도 같은 두 엔진)로 줄인다.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions, LegalComment};
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct Options {
    /// JS 변수 이름 줄이기
    ///
    /// JS 파일에서 함수 안 지역 변수·매개변수 이름을 `a`, `b`처럼 짧게 바꿉니다. 전역 이름과 내보낸(export) 이름은 그대로 둡니다.
    mangle: bool,
    /// 라이선스 주석 남기기
    ///
    /// JS의 `/*! … */`·`//!`·`@license`·`@preserve` 주석과 CSS 파일 맨 앞의 `/*! … */` 주석을 남깁니다. 그 밖의 주석은 늘 지웁니다.
    keep_license: bool,
    /// HTML 안의 CSS·JS도 압축
    ///
    /// HTML 파일의 `<style>`, `style` 속성, `<script>` 내용도 줄입니다. `<script>` 안의 변수 이름은 줄이지 않습니다. 끄면 태그 사이 공백과 주석만 줄입니다.
    minify_inline: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mangle: true,
            keep_license: true,
            minify_inline: true,
        }
    }
}

struct MiscMinify;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscMinify)
}

waple_tools_addon_abi::export_tool!(tool);

/// `src`의 바이트 위치 → 1부터 센 (줄, 열)
fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let before = &src[..src.floor_char_boundary(offset)];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}

fn syntax_error(pos: Option<(usize, usize)>, msg: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(match pos {
        Some((line, col)) => format!("구문 오류({line}줄 {col}열): {msg}"),
        None => format!("구문 오류: {msg}"),
    })
}

fn css(src: &str, o: &Options) -> Result<String, ToolError> {
    // lightningcss의 줄은 0부터, 열은 1부터 센다
    let mut sheet = StyleSheet::parse(src, ParserOptions::default()).map_err(|e| {
        syntax_error(
            e.loc.map(|l| (l.line as usize + 1, l.column as usize)),
            &e.kind,
        )
    })?;
    if !o.keep_license {
        sheet.license_comments.clear();
    }
    sheet
        .minify(MinifyOptions::default())
        .map_err(|e| ToolError::Failed(format!("CSS를 줄이지 못했습니다: {}", e.kind)))?;
    let out = sheet
        .to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|e| ToolError::Failed(format!("CSS를 쓰지 못했습니다: {}", e.kind)))?;
    Ok(out.code)
}

fn js(src: &str, source_type: SourceType, o: &Options) -> Result<String, ToolError> {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, src, source_type).parse();
    if let Some(e) = parsed.errors.first() {
        let pos = e
            .labels
            .as_ref()
            .and_then(|l| l.first())
            .map(|l| line_col(src, l.offset()));
        return Err(syntax_error(pos, &e.message));
    }
    let mut program = parsed.program;
    // safest: 쓰지 않는 듯한 코드를 지우지 않고 함수·클래스 이름(name)을 지킨다
    let ret = Minifier::new(MinifierOptions {
        mangle: o.mangle.then(MangleOptions::default),
        compress: Some(CompressOptions::safest()),
    })
    .minify(&alloc, &mut program);
    let legal = if o.keep_license {
        LegalComment::Inline
    } else {
        LegalComment::None
    };
    Ok(Codegen::new()
        .with_options(CodegenOptions {
            comments: CommentOptions {
                legal,
                ..CommentOptions::disabled()
            },
            ..CodegenOptions::minify()
        })
        .with_scoping(ret.scoping)
        .with_private_member_mappings(ret.class_private_mappings)
        .build(&program)
        .code)
}

fn html(src: &str, o: &Options) -> Vec<u8> {
    let cfg = minify_html::Cfg {
        minify_css: o.minify_inline,
        minify_js: o.minify_inline,
        ..minify_html::Cfg::default()
    };
    minify_html::minify(src.as_bytes(), &cfg)
}

/// 확장자에 맞게 줄인 내용
fn minify(src: &str, ext: &str, o: &Options) -> Result<Vec<u8>, ToolError> {
    Ok(match ext {
        "css" => css(src, o)?.into_bytes(),
        "mjs" => js(src, SourceType::mjs(), o)?.into_bytes(),
        "cjs" => js(src, SourceType::cjs(), o)?.into_bytes(),
        // .js는 import·export가 있으면 모듈, 없으면 스크립트로 읽는다.
        // unambiguous는 최상위 await를 스크립트로 보고 오류를 내서 모듈로 한 번 더 읽어 본다
        "js" => js(src, SourceType::unambiguous(), o)
            .or_else(|e| js(src, SourceType::mjs(), o).map_err(|_| e))?
            .into_bytes(),
        "html" | "htm" => html(src, o),
        _ => {
            return Err(ToolError::Failed(
                "CSS·JS·HTML 파일만 압축할 수 있습니다.".into(),
            ));
        }
    })
}

impl Tool for MiscMinify {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.minify".into(),
            name: "CSS · JS · HTML 압축".into(),
            category: "misc".into(),
            description: "공백·주석을 줄여 CSS·JS·HTML 파일 용량을 줄입니다.".into(),
            extensions: ["css", "js", "mjs", "cjs", "html", "htm"]
                .map(String::from)
                .to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(Options).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("CSS·JS·HTML 파일을 넣어 주세요.".into()))?;
        ctx.check_cancel()?;
        let o: Options = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        let bytes = std::fs::read(src)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            ToolError::Failed(
                "UTF-8 텍스트가 아닙니다. 다른 인코딩(EUC-KR 등)으로 저장된 파일은 UTF-8로 바꿔 넣어 주세요."
                    .into(),
            )
        })?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let out = minify(text, &ext, &o)?;
        ctx.check_cancel()?;
        let rel = PathBuf::from(src.file_name().unwrap_or_default());
        std::fs::write(out_dir.join(&rel), out)?;
        Ok(ToolOutput::Files(vec![rel]))
    }
}
