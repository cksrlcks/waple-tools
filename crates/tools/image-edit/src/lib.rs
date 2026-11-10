//! 자르기 · 회전 · 여백 (기본 내장): 회전·뒤집기 → 크롭 → 여백

mod crop;
mod lossless;
mod margin;
#[cfg(test)]
mod tests;

use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use image::{DynamicImage, Rgba};
use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, INPUT_EXTS, MetaMode, Quality, encode, flatten, load, parse_color, parse_options,
    run_each, schema_of, with_meta, write_output,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::lossless::{FLIP_H, FLIP_V, ROT_90, ROT_180, ROT_270, lossless_jpeg};

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Angle {
    /// 회전 안 함
    #[default]
    None,
    /// 90° (시계 방향)
    R90,
    /// 180°
    R180,
    /// 270° (시계 반대 90°)
    R270,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum CropMode {
    /// 자르지 않음
    #[default]
    None,
    /// 비율
    Ratio,
    /// 픽셀
    Pixels,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Ratio {
    /// 1:1
    #[default]
    R1x1,
    /// 4:3
    R4x3,
    /// 3:4
    R3x4,
    /// 16:9
    R16x9,
    /// 9:16
    R9x16,
    /// 3:2
    R3x2,
    /// 2:3
    R2x3,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Anchor {
    /// 가운데
    #[default]
    Center,
    /// 위
    Top,
    /// 아래
    Bottom,
    /// 왼쪽
    Left,
    /// 오른쪽
    Right,
    /// 왼쪽 위
    TopLeft,
    /// 오른쪽 위
    TopRight,
    /// 왼쪽 아래
    BottomLeft,
    /// 오른쪽 아래
    BottomRight,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum MarginMode {
    /// 그대로
    #[default]
    None,
    /// 여백 자동 자르기
    Trim,
    /// 여백 추가
    Pad,
    /// 정사각형 캔버스
    Square,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct EditOptions {
    /// 회전
    #[schemars(extend("x-group" = "회전"))]
    angle: Angle,
    /// 좌우 뒤집기
    #[schemars(extend("x-group" = "회전"))]
    flip_h: bool,
    /// 상하 뒤집기
    #[schemars(extend("x-group" = "회전"))]
    flip_v: bool,

    /// 크롭
    #[schemars(extend("x-group" = "크롭"))]
    crop: CropMode,
    /// 비율
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "crop": ["ratio"] }))]
    ratio: Ratio,
    /// 가로(px)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "crop": ["pixels"] }))]
    width: u32,
    /// 세로(px)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "crop": ["pixels"] }))]
    height: u32,
    /// 기준 위치
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "crop": ["ratio", "pixels"] }))]
    anchor: Anchor,

    /// 여백
    #[schemars(extend("x-group" = "여백"))]
    margin: MarginMode,
    /// 허용 오차(%)
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "여백", "x-enabled-when" = { "margin": ["trim"] }))]
    tolerance: u8,
    /// 여백(px)
    #[schemars(range(min = 0))]
    #[schemars(extend("x-group" = "여백", "x-enabled-when" = { "margin": ["pad"] }))]
    padding: u32,
    /// 여백을 투명하게 (PNG·WebP)
    #[schemars(extend("x-group" = "여백", "x-enabled-when" = { "margin": ["pad", "square"] }))]
    transparent: bool,
    /// 여백 색
    #[schemars(extend("format" = "color", "x-group" = "여백", "x-enabled-when" = { "margin": ["pad", "square"] }))]
    background: String,

    /// 품질 (JPG·WebP, 다시 저장할 때)
    #[schemars(range(min = 1, max = 100))]
    #[schemars(extend("x-group" = "저장"))]
    quality: u8,
    /// 메타데이터
    #[schemars(extend("x-group" = "저장"))]
    metadata: MetaMode,
}

impl Default for EditOptions {
    fn default() -> Self {
        Self {
            angle: Angle::None,
            flip_h: false,
            flip_v: false,
            crop: CropMode::None,
            ratio: Ratio::R1x1,
            width: 1000,
            height: 1000,
            anchor: Anchor::Center,
            margin: MarginMode::None,
            tolerance: 10,
            padding: 20,
            transparent: true,
            background: "#ffffff".into(),
            quality: 95,
            metadata: MetaMode::Strip,
        }
    }
}

impl EditOptions {
    /// 회전·뒤집기를 C 래퍼 op 번호로 (회전 후 뒤집기 순서)
    fn ops(&self) -> Vec<c_int> {
        let mut v = Vec::new();
        match self.angle {
            Angle::None => {}
            Angle::R90 => v.push(ROT_90),
            Angle::R180 => v.push(ROT_180),
            Angle::R270 => v.push(ROT_270),
        }
        if self.flip_h {
            v.push(FLIP_H);
        }
        if self.flip_v {
            v.push(FLIP_V);
        }
        v
    }

    fn is_noop(&self) -> bool {
        self.ops().is_empty() && self.crop == CropMode::None && self.margin == MarginMode::None
    }
}

fn edit(input: &Path, out_dir: &Path, o: &EditOptions, bg: [u8; 3]) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let fmt = src.format;
    let keep = o.metadata == MetaMode::Keep;
    // 회전·뒤집기만 있는 JPG는 가능하면 무손실로
    let geometry_only = o.crop == CropMode::None && o.margin == MarginMode::None;
    if geometry_only && let Some(bytes) = lossless_jpeg(input, &src, &o.ops(), keep) {
        return write_output(out_dir, input, fmt, &bytes);
    }

    let mut img: DynamicImage = match o.angle {
        Angle::None => src.image,
        Angle::R90 => src.image.rotate90(),
        Angle::R180 => src.image.rotate180(),
        Angle::R270 => src.image.rotate270(),
    };
    if o.flip_h {
        img = img.fliph();
    }
    if o.flip_v {
        img = img.flipv();
    }
    let (x, y, w, h) = crop::crop_rect(img.width(), img.height(), o);
    if (w, h) != (img.width(), img.height()) {
        img = img.crop_imm(x, y, w, h);
    }
    let alpha_ok = matches!(fmt, Fmt::Png | Fmt::Webp | Fmt::Gif);
    let fill = if alpha_ok && o.transparent {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([bg[0], bg[1], bg[2], 255])
    };
    img = match o.margin {
        MarginMode::None => img,
        MarginMode::Trim => margin::trim(img, o.tolerance),
        MarginMode::Pad => margin::pad(img, o.padding, fill),
        MarginMode::Square => margin::square(img, fill),
    };
    if fmt == Fmt::Jpeg && img.color().has_alpha() {
        img = flatten(&img, bg);
    }
    let q = Quality {
        quality: o.quality.clamp(1, 100),
        lossless: false,
    };
    let mut bytes = encode(&img, fmt, q)?;
    if keep {
        bytes = with_meta(bytes, fmt, &src.meta)?;
    }
    write_output(out_dir, input, fmt, &bytes)
}

struct Edit;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Edit)
}

impl Tool for Edit {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.edit".into(),
            name: "자르기 · 회전 · 여백".into(),
            category: "image".into(),
            description: "회전·뒤집기, 크롭, 여백 자르기·추가를 합니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<EditOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: EditOptions = parse_options(opts)?;
        if o.is_noop() {
            return Err(ToolError::InvalidOptions(
                "회전·크롭·여백 중 하나 이상 고르세요.".into(),
            ));
        }
        let bg = parse_color(&o.background).ok_or_else(|| {
            ToolError::InvalidOptions("여백 색은 #rrggbb 형식이어야 합니다.".into())
        })?;
        run_each(ctx, input, |p| edit(p, out_dir, &o, bg))
    }
}
