//! 자르기 · 회전 (기본 내장): 회전·뒤집기 → 크롭

mod crop;
mod lossless;
#[cfg(test)]
mod tests;

use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use image::DynamicImage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    INPUT_EXTS, MetaMode, Quality, apply_meta, encode, load, parse_options, run_each, schema_of,
    write_output,
};

use crate::lossless::{FLIP_H, FLIP_V, ROT_90, ROT_180, ROT_270, lossless_jpeg};

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Angle {
    /// 회전 안 함
    #[default]
    None,
    /// 90°
    R90,
    /// 180°
    R180,
    /// 270°
    R270,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum CropMode {
    /// 직접 지정
    #[default]
    Manual,
    /// 픽셀
    Pixels,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Ratio {
    /// 자유
    #[default]
    Free,
    /// 1:1
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

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-preview" = "result",
    // 한 번에 파일 하나만: 미리보기에서 영역을 직접 지정하므로
    "x-single-input" = true,
    // 미리보기 위에서 끌어 지정하는 영역 (회전·뒤집기 뒤 이미지 기준 px)
    "x-region" = { "x": "cropX", "y": "cropY", "w": "width", "h": "height", "ratio": "ratio", "angle": "angle", "flipH": "flipH", "flipV": "flipV", "when": { "crop": [true], "cropMode": ["manual"] } },
    // 세 묶음 스위치 중 하나는 켜야 실행 (이미지 최적화와 같은 방식)
    "x-require-any" = ["rotate", "crop"],
    "x-require-message" = "회전·자르기 중 하나 이상 켜세요."
))]
struct EditOptions {
    /// 회전·뒤집기
    #[schemars(extend("x-group" = "회전", "x-group-toggle" = true))]
    rotate: bool,
    /// 회전
    #[schemars(extend("x-group" = "회전"))]
    angle: Angle,
    /// 좌우 뒤집기
    #[schemars(extend("x-group" = "회전"))]
    flip_h: bool,
    /// 상하 뒤집기
    #[schemars(extend("x-group" = "회전"))]
    flip_v: bool,

    /// 자르기
    #[schemars(extend("x-group" = "크롭", "x-group-toggle" = true))]
    crop: bool,
    /// 방식
    #[schemars(extend("x-group" = "크롭"))]
    crop_mode: CropMode,
    /// 비율
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["manual"] }))]
    ratio: Ratio,
    /// 가로(px)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["pixels", "manual"] }))]
    width: u32,
    /// 세로(px)
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["pixels", "manual"] }))]
    height: u32,
    /// 왼쪽에서(px)
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["manual"] }))]
    crop_x: u32,
    /// 위에서(px)
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["manual"] }))]
    crop_y: u32,
    /// 기준 위치
    #[schemars(extend("x-group" = "크롭", "x-enabled-when" = { "cropMode": ["pixels"] }))]
    anchor: Anchor,

    /// 품질 (JPG·WebP, 다시 저장할 때)
    ///
    /// 이미지를 새로 저장하므로 품질 100이어도 용량이 달라질 수 있습니다. PNG는 화질 손실 없이 압축 방식만 달라지고, 메타데이터를 지우면 그만큼 줄어듭니다. JPG를 회전·뒤집기만 하면 가능한 경우 무손실로 처리합니다.
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
            rotate: false,
            angle: Angle::None,
            flip_h: false,
            flip_v: false,
            crop: false,
            crop_mode: CropMode::Manual,
            ratio: Ratio::Free,
            width: 1000,
            height: 1000,
            crop_x: 0,
            crop_y: 0,
            anchor: Anchor::Center,
            quality: 100,
            metadata: MetaMode::Strip,
        }
    }
}

impl EditOptions {
    /// 회전·뒤집기를 C 래퍼 op 번호로 (회전 후 뒤집기 순서)
    fn ops(&self) -> Vec<c_int> {
        let mut v = Vec::new();
        if !self.rotate {
            return v;
        }
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
        self.ops().is_empty() && !self.crop
    }
}

fn edit(input: &Path, out_dir: &Path, o: &EditOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let fmt = src.format;
    let keep = o.metadata == MetaMode::Keep;
    // 회전·뒤집기만 있는 JPG는 가능하면 무손실로
    let geometry_only = !o.crop;
    if geometry_only && let Some(bytes) = lossless_jpeg(input, &src, &o.ops(), keep) {
        return write_output(out_dir, input, fmt, &bytes);
    }

    let angle = if o.rotate { o.angle } else { Angle::None };
    let mut img: DynamicImage = match angle {
        Angle::None => src.image,
        Angle::R90 => src.image.rotate90(),
        Angle::R180 => src.image.rotate180(),
        Angle::R270 => src.image.rotate270(),
    };
    if o.rotate && o.flip_h {
        img = img.fliph();
    }
    if o.rotate && o.flip_v {
        img = img.flipv();
    }
    let (x, y, w, h) = crop::crop_rect(img.width(), img.height(), o);
    if (w, h) != (img.width(), img.height()) {
        img = img.crop_imm(x, y, w, h);
    }
    let q = Quality {
        quality: o.quality.clamp(1, 100),
        lossless: false,
    };
    let mut bytes = encode(&img, fmt, q)?;
    bytes = apply_meta(bytes, fmt, &src.meta, o.metadata)?;
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
            name: "자르기 · 회전".into(),
            category: "image".into(),
            description: "회전·뒤집기와 자르기를 합니다.".into(),
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
                "회전·자르기 중 하나 이상 켜세요.".into(),
            ));
        }
        run_each(ctx, input, |p| edit(p, out_dir, &o))
    }
}
