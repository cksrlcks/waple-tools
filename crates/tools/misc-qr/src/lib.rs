//! QR 코드 생성 (추가 설치, Interactive, 입력 파일 없음): URL → PNG 이미지 + SVG 코드 (복사·저장)

use std::path::{Path, PathBuf};

use base64::Engine as _;
use qrcode::{Color, EcLevel, QrCode};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{Block, JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Ec {
    /// 낮음 (7%)
    Low,
    /// 보통 (15%)
    #[default]
    Medium,
    /// 높음 (25%)
    Quartile,
    /// 최고 (30%)
    High,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-input" = "none"))]
struct QrOptions {
    /// URL
    ///
    /// QR에 담을 주소입니다. 적은 글자를 그대로 담습니다.
    #[schemars(extend("x-wide" = true))]
    url: String,
    /// 크기(px)
    ///
    /// PNG 한 변의 길이입니다. 칸이 정확히 나뉘도록 조금 작아질 수 있습니다.
    #[schemars(range(min = 128, max = 2048))]
    size: u32,
    /// 여백(칸)
    ///
    /// QR 바깥의 빈 칸 수입니다. 표준은 4칸이며, 너무 좁으면 못 읽는 카메라가 있습니다.
    #[schemars(range(min = 0, max = 10))]
    margin: u32,
    /// 오류 복원
    ///
    /// 높을수록 일부가 가려지거나 인쇄가 번져도 읽히지만 무늬가 촘촘해집니다.
    error_correction: Ec,
    /// 전경색
    #[schemars(extend("format" = "color"))]
    foreground: String,
    /// 배경색
    #[schemars(extend("format" = "color"))]
    background: String,
}

impl Default for QrOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            size: 512,
            margin: 4,
            error_correction: Ec::Medium,
            foreground: "#000000".into(),
            background: "#ffffff".into(),
        }
    }
}

struct MiscQr;

pub fn tool() -> Box<dyn Tool> {
    Box::new(MiscQr)
}

waple_tools_addon_abi::export_tool!(tool);

fn parse_color(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([ch(0)?, ch(2)?, ch(4)?])
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// 밝기 (0~255, ITU-R BT.601)
fn luma([r, g, b]: [u8; 3]) -> u32 {
    (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000
}

/// QR 행렬 (여백 포함 한 변 칸 수, 칸마다 어두운지)
struct Matrix {
    n: u32,
    margin: u32,
    width: u32,
    dark: Vec<bool>,
}

impl Matrix {
    fn new(code: &QrCode, margin: u32) -> Self {
        let width = code.width() as u32;
        let dark = code
            .to_colors()
            .into_iter()
            .map(|c| c == Color::Dark)
            .collect();
        Self {
            n: width + 2 * margin,
            margin,
            width,
            dark,
        }
    }

    /// 여백 포함 좌표 (칸 단위)
    fn is_dark(&self, x: u32, y: u32) -> bool {
        let (m, w) = (self.margin, self.width);
        x >= m && y >= m && x < m + w && y < m + w && self.dark[((y - m) * w + (x - m)) as usize]
    }
}

fn png(m: &Matrix, cell: u32, fg: [u8; 3], bg: [u8; 3]) -> Result<Vec<u8>, ToolError> {
    let side = m.n * cell;
    let mut data = Vec::with_capacity((side * side * 3) as usize);
    for y in 0..side {
        for x in 0..side {
            data.extend(if m.is_dark(x / cell, y / cell) {
                fg
            } else {
                bg
            });
        }
    }
    let fail = |e: png::EncodingError| ToolError::Failed(format!("PNG를 만들 수 없습니다: {e}"));
    let mut buf = Vec::new();
    let mut enc = png::Encoder::new(&mut buf, side, side);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(fail)?;
    w.write_image_data(&data).map_err(fail)?;
    w.finish().map_err(fail)?;
    Ok(buf)
}

/// 한 줄의 이어진 어두운 칸을 사각형 하나로 묶은 path
fn svg(m: &Matrix, side: u32, fg: [u8; 3], bg: [u8; 3]) -> String {
    let mut d = String::new();
    for y in 0..m.n {
        let mut x = 0;
        while x < m.n {
            if !m.is_dark(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < m.n && m.is_dark(x, y) {
                x += 1;
            }
            let len = x - start;
            d.push_str(&format!("M{start} {y}h{len}v1h-{len}z"));
        }
    }
    let n = m.n;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {n} {n}\" width=\"{side}\" height=\"{side}\" shape-rendering=\"crispEdges\">\
<rect width=\"{n}\" height=\"{n}\" fill=\"{}\"/><path fill=\"{}\" d=\"{d}\"/></svg>",
        hex(bg),
        hex(fg)
    )
}

fn build(o: &QrOptions) -> Result<Vec<Block>, ToolError> {
    let url = o.url.trim();
    if url.is_empty() {
        return Ok(vec![Block::Text {
            text: "URL을 입력해 주세요.".into(),
        }]);
    }
    let color = |s: &str, what: &str| {
        parse_color(s).ok_or_else(|| {
            ToolError::InvalidOptions(format!("{what}은 #rrggbb 형식이어야 합니다."))
        })
    };
    let fg = color(&o.foreground, "전경색")?;
    let bg = color(&o.background, "배경색")?;
    let ec = match o.error_correction {
        Ec::Low => EcLevel::L,
        Ec::Medium => EcLevel::M,
        Ec::Quartile => EcLevel::Q,
        Ec::High => EcLevel::H,
    };
    let code = QrCode::with_error_correction_level(url.as_bytes(), ec).map_err(|_| {
        ToolError::Failed("URL이 너무 깁니다. 줄이거나 오류 복원 수준을 낮춰 주세요.".into())
    })?;
    let m = Matrix::new(&code, o.margin.min(10));
    // 칸이 정확히 나뉘는 가장 큰 크기 (너무 촘촘하면 칸당 1px)
    let cell = (o.size.clamp(128, 2048) / m.n).max(1);
    let side = m.n * cell;

    let mut blocks = vec![];
    if luma(fg) + 64 > luma(bg) {
        blocks.push(Block::Text {
            text: "전경색이 배경색보다 충분히 어둡지 않습니다. 이대로면 못 읽는 카메라가 많습니다."
                .into(),
        });
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(png(&m, cell, fg, bg)?);
    blocks.push(Block::Image {
        src: format!("data:image/png;base64,{b64}"),
        name: "qr.png".into(),
    });
    blocks.push(Block::Number {
        label: "크기".into(),
        value: side as f64,
        unit: Some("px".into()),
    });
    blocks.push(Block::Code {
        lang: "svg".into(),
        code: svg(&m, side, fg, bg),
        name: Some("qr.svg".into()),
    });
    Ok(blocks)
}

impl Tool for MiscQr {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "misc.qr".into(),
            name: "QR 코드 생성".into(),
            category: "misc".into(),
            description: "URL을 QR 코드 PNG·SVG로 만듭니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Interactive
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(QrOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        _input: &[PathBuf],
        _out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: QrOptions = serde_json::from_value(if opts.is_null() {
            serde_json::json!({})
        } else {
            opts.clone()
        })
        .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        ctx.check_cancel()?;
        build(&opts).map(ToolOutput::Blocks)
    }
}

#[cfg(test)]
mod tests;
