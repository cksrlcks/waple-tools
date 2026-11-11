//! 워터마크 (추가 설치, PerFile): 텍스트나 로고를 9방향 중 한 곳에 넣거나 비스듬히 반복(타일)한다

mod draw;
mod mark;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use image::{ColorType, DynamicImage, RgbaImage};
use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    INPUT_EXTS, MetaMode, Quality, apply_meta, encode, ensure_pixels, load, parse_color,
    parse_options, run_each, schema_of, write_output,
};

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Kind {
    /// 텍스트
    #[default]
    Text,
    /// 로고 이미지
    Logo,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Font {
    /// 맑은 고딕
    #[default]
    MalgunGothic,
    /// 맑은 고딕 굵게
    MalgunGothicBold,
    /// 굴림
    Gulim,
    /// 바탕
    Batang,
    /// Arial (영문)
    Arial,
    /// Arial 굵게 (영문)
    ArialBold,
}

impl Font {
    /// Windows 글꼴 폴더 안의 파일 이름
    fn file(self) -> &'static str {
        match self {
            Font::MalgunGothic => "malgun.ttf",
            Font::MalgunGothicBold => "malgunbd.ttf",
            Font::Gulim => "gulim.ttc",
            Font::Batang => "batang.ttc",
            Font::Arial => "arial.ttf",
            Font::ArialBold => "arialbd.ttf",
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
enum Position {
    /// 왼쪽 위
    TopLeft,
    /// 위 가운데
    Top,
    /// 오른쪽 위
    TopRight,
    /// 왼쪽 가운데
    Left,
    /// 가운데
    Center,
    /// 오른쪽 가운데
    Right,
    /// 왼쪽 아래
    BottomLeft,
    /// 아래 가운데
    Bottom,
    /// 오른쪽 아래
    #[default]
    BottomRight,
    /// 타일 (전체에 반복)
    Tile,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    // 글자·로고를 얹고 다시 저장하면 원본보다 커질 수 있다
    "x-allow-larger" = true
))]
struct WatermarkOptions {
    /// 종류
    #[schemars(extend("x-group" = "워터마크"))]
    kind: Kind,
    /// 텍스트
    #[schemars(extend("x-group" = "워터마크", "x-enabled-when" = { "kind": ["text"] }))]
    text: String,
    /// 글꼴
    ///
    /// Windows에 설치된 글꼴을 씁니다. Arial에는 한글이 없습니다.
    #[schemars(extend("x-group" = "워터마크", "x-enabled-when" = { "kind": ["text"] }))]
    font: Font,
    /// 글자 크기(%)
    ///
    /// 이미지 짧은 변 대비 글자 높이입니다. 해상도가 달라도 비슷한 크기로 보입니다.
    #[schemars(range(min = 1, max = 30))]
    #[schemars(extend("x-group" = "워터마크", "x-enabled-when" = { "kind": ["text"] }))]
    text_size: u8,
    /// 글자 색
    #[schemars(extend("format" = "color", "x-group" = "워터마크", "x-enabled-when" = { "kind": ["text"] }))]
    color: String,
    /// 로고 파일
    ///
    /// 배경이 투명한 PNG가 가장 잘 어울립니다.
    #[schemars(extend(
        "format" = "file",
        "x-file-extensions" = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"],
        "x-group" = "워터마크",
        "x-enabled-when" = { "kind": ["logo"] }
    ))]
    logo: String,
    /// 로고 크기(%)
    ///
    /// 이미지 짧은 변 대비 로고 가로 길이입니다. 이미지보다 크면 맞춰 줄입니다.
    #[schemars(range(min = 1, max = 100))]
    #[schemars(extend("x-group" = "워터마크", "x-enabled-when" = { "kind": ["logo"] }))]
    logo_size: u8,

    /// 위치
    ///
    /// 타일은 워터마크를 비스듬히(30°) 기울여 이미지 전체에 반복합니다.
    #[schemars(extend("x-group" = "배치"))]
    position: Position,
    /// 여백(%)
    ///
    /// 가장자리에서 띄울 거리입니다 (이미지 짧은 변 대비).
    #[schemars(range(min = 0, max = 20))]
    #[schemars(extend("x-group" = "배치", "x-enabled-when" = {
        "position": ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"]
    }))]
    margin: u8,
    /// 불투명도(%)
    ///
    /// 100이면 또렷하게, 낮을수록 투명하게 비칩니다.
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "배치"))]
    opacity: u8,

    /// 메타데이터
    #[schemars(extend("x-group" = "저장"))]
    metadata: MetaMode,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            kind: Kind::Text,
            text: "© 워터마크".into(),
            font: Font::MalgunGothic,
            text_size: 5,
            color: "#ffffff".into(),
            logo: String::new(),
            logo_size: 20,
            position: Position::BottomRight,
            margin: 3,
            opacity: 50,
            metadata: MetaMode::Strip,
        }
    }
}

/// run() 한 번에 한 번 읽는 워터마크 재료. PerFile 엔진은 파일마다 run()을 부르므로 실제로는 파일마다 다시 읽는다
enum Source {
    Text {
        font: ab_glyph::FontVec,
        color: [u8; 3],
    },
    Logo(RgbaImage),
}

/// 다시 저장할 때 화질 손실을 줄이는 JPEG·WebP 품질
const QUALITY: u8 = 92;

fn watermark(
    input: &Path,
    out_dir: &Path,
    o: &WatermarkOptions,
    source: &Source,
) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (w, h) = (src.image.width(), src.image.height());
    ensure_pixels(w, h)?;
    let had_alpha = src.image.color().has_alpha();

    let short = w.min(h) as f32;
    let margin = if o.position == Position::Tile {
        0
    } else {
        (short * f32::from(o.margin.min(20)) / 100.0).round() as u32
    };
    // 들어갈 칸: 이미지에서 양쪽 여백을 뺀 크기 (넘치면 줄인다)
    let (box_w, box_h) = (
        w.saturating_sub(2 * margin).max(1),
        h.saturating_sub(2 * margin).max(1),
    );
    let m = match source {
        Source::Text { font, color } => {
            let px = short * f32::from(o.text_size.clamp(1, 30)) / 100.0;
            mark::text(font, &o.text, px, *color, box_w, box_h)?
        }
        Source::Logo(logo) => {
            let width = short * f32::from(o.logo_size.clamp(1, 100)) / 100.0;
            mark::logo(logo, width, box_w, box_h)
        }
    };
    let opacity = f32::from(o.opacity.min(100)) / 100.0;
    // 16비트 PNG·TIFF는 16비트로 그리고 저장한다
    let deep = matches!(
        src.image.color(),
        ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16
    );
    let out = if deep {
        let mut img = src.image.to_rgba16();
        draw::stamp(&mut img, &m, o.position, margin, opacity);
        let img = DynamicImage::ImageRgba16(img);
        if had_alpha {
            img
        } else {
            DynamicImage::ImageRgb16(img.to_rgb16())
        }
    } else {
        let mut img = src.image.to_rgba8();
        draw::stamp(&mut img, &m, o.position, margin, opacity);
        let img = DynamicImage::ImageRgba8(img);
        if had_alpha {
            img
        } else {
            DynamicImage::ImageRgb8(img.to_rgb8())
        }
    };
    // shortcut: 무손실 WebP 원본도 손실(품질 92)로 저장한다. 원본 압축 방식을 알 수 있게 되면 맞춘다
    let bytes = encode(
        &out,
        src.format,
        Quality {
            quality: QUALITY,
            lossless: false,
        },
    )?;
    let bytes = apply_meta(bytes, src.format, &src.meta, o.metadata)?;
    write_output(out_dir, input, src.format, &bytes)
}

struct ImageWatermark;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageWatermark)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for ImageWatermark {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.watermark".into(),
            name: "워터마크".into(),
            category: "image".into(),
            description: "텍스트나 로고를 이미지에 넣습니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<WatermarkOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: WatermarkOptions = parse_options(opts)?;
        let source = match o.kind {
            Kind::Text => {
                if o.text.trim().is_empty() {
                    return Err(ToolError::InvalidOptions(
                        "넣을 텍스트를 입력해 주세요.".into(),
                    ));
                }
                let color = parse_color(&o.color).ok_or_else(|| {
                    ToolError::InvalidOptions("글자 색은 #rrggbb 형식이어야 합니다.".into())
                })?;
                Source::Text {
                    font: mark::system_font(o.font.file())?,
                    color,
                }
            }
            Kind::Logo => Source::Logo(mark::load_logo(&o.logo)?),
        };
        run_each(ctx, input, |p| watermark(p, out_dir, &o, &source))
    }
}
