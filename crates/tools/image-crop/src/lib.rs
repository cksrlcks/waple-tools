//! 크롭 (기본 내장): 비율 프리셋 또는 픽셀 크기, 기준 위치

use std::path::{Path, PathBuf};

use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    INPUT_EXTS, MetaMode, Quality, encode, load, parse_options, run_each, schema_of, with_meta,
    write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 비율
    #[default]
    Ratio,
    /// 픽셀
    Pixels,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
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

impl Ratio {
    fn parts(self) -> (u32, u32) {
        match self {
            Ratio::R1x1 => (1, 1),
            Ratio::R4x3 => (4, 3),
            Ratio::R3x4 => (3, 4),
            Ratio::R16x9 => (16, 9),
            Ratio::R9x16 => (9, 16),
            Ratio::R3x2 => (3, 2),
            Ratio::R2x3 => (2, 3),
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
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
struct CropOptions {
    /// 방식
    mode: Mode,
    /// 비율
    ratio: Ratio,
    /// 가로(px)
    #[schemars(range(min = 1))]
    width: u32,
    /// 세로(px)
    #[schemars(range(min = 1))]
    height: u32,
    /// 기준 위치
    anchor: Anchor,
    /// 품질 (JPG·WebP)
    #[schemars(range(min = 1, max = 100))]
    quality: u8,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for CropOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Ratio,
            ratio: Ratio::R1x1,
            width: 1000,
            height: 1000,
            anchor: Anchor::Center,
            quality: 95,
            metadata: MetaMode::Strip,
        }
    }
}

/// 자를 영역 (x, y, 너비, 높이). 비율은 원본 안에 들어가는 가장 큰 상자, 픽셀은 원본 크기로 자른다.
fn crop_rect(w: u32, h: u32, o: &CropOptions) -> (u32, u32, u32, u32) {
    let (cw, ch) = match o.mode {
        Mode::Pixels => (o.width.clamp(1, w), o.height.clamp(1, h)),
        Mode::Ratio => {
            let (rw, rh) = o.ratio.parts();
            // 가로를 꽉 채웠을 때 세로가 넘치면 세로에 맞춘다
            let by_width = (w, (u64::from(w) * u64::from(rh) / u64::from(rw)) as u32);
            if by_width.1 <= h {
                by_width
            } else {
                ((u64::from(h) * u64::from(rw) / u64::from(rh)) as u32, h)
            }
        }
    };
    let (sx, sy) = (w - cw, h - ch);
    let (x, y) = match o.anchor {
        Anchor::Center => (sx / 2, sy / 2),
        Anchor::Top => (sx / 2, 0),
        Anchor::Bottom => (sx / 2, sy),
        Anchor::Left => (0, sy / 2),
        Anchor::Right => (sx, sy / 2),
        Anchor::TopLeft => (0, 0),
        Anchor::TopRight => (sx, 0),
        Anchor::BottomLeft => (0, sy),
        Anchor::BottomRight => (sx, sy),
    };
    (x, y, cw, ch)
}

fn crop(input: &Path, out_dir: &Path, o: &CropOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let (x, y, cw, ch) = crop_rect(src.image.width(), src.image.height(), o);
    let img = src.image.crop_imm(x, y, cw, ch);
    let q = Quality {
        quality: o.quality.clamp(1, 100),
        lossless: false,
    };
    let mut bytes = encode(&img, src.format, q)?;
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, src.format, &src.meta)?;
    }
    write_output(out_dir, input, src.format, &bytes)
}

struct Crop;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Crop)
}

impl Tool for Crop {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.crop".into(),
            name: "크롭".into(),
            category: "image".into(),
            description: "비율이나 픽셀 크기로 잘라냅니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<CropOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: CropOptions = parse_options(opts)?;
        run_each(ctx, input, |p| crop(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, ToolOutput};
    use localtools_image_common::testkit::photo;
    use serde_json::json;

    fn opts(v: serde_json::Value) -> CropOptions {
        localtools_image_common::parse_options(&v).unwrap()
    }

    #[test]
    fn crop_rect_table() {
        let r = |w, h, v| crop_rect(w, h, &opts(v));
        assert_eq!(
            r(400, 300, json!({"ratio": "r1x1", "anchor": "center"})),
            (50, 0, 300, 300)
        );
        assert_eq!(
            r(400, 300, json!({"ratio": "r16x9", "anchor": "top"})),
            (0, 0, 400, 225)
        );
        assert_eq!(
            r(
                400,
                300,
                json!({"mode": "pixels", "width": 500, "height": 100, "anchor": "bottomRight"})
            ),
            (0, 200, 400, 100)
        );
        assert_eq!(
            r(
                400,
                300,
                json!({"mode": "pixels", "width": 100, "height": 100, "anchor": "topLeft"})
            ),
            (0, 0, 100, 100)
        );
        assert_eq!(
            r(300, 400, json!({"ratio": "r4x3", "anchor": "bottom"})),
            (0, 175, 300, 225)
        );
    }

    #[test]
    fn crop_writes_box_size() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(400, 300).save(&src).unwrap();
        let out = d.path().join("o");
        std::fs::create_dir(&out).unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        let ToolOutput::Files(f) = tool()
            .run(&ctx, &[src], &out, &json!({"ratio": "r16x9"}))
            .unwrap()
        else {
            panic!()
        };
        let img = image::open(out.join(&f[0])).unwrap();
        assert_eq!((img.width(), img.height()), (400, 225));
        assert_eq!(tool().meta().name, "크롭");
    }
}
