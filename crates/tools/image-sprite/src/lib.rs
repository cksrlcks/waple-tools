//! 스프라이트 생성 (추가 설치, Combine): 넣은 순서대로 이미지 여러 장 → `sprite/` 폴더 (sprite.png + sprite.css, 2x면 sprite@2x.png)

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{DynamicImage, RgbaImage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{
    Fmt, INPUT_EXTS, Quality, encode, ensure_pixels, load, parse_options, resize_rgba, schema_of,
};

/// 결과 폴더. 한 폴더에 묶어야 이름이 겹쳐 `(1)`이 붙어도 CSS의 url(sprite.png)이 맞는다
const DIR: &str = "sprite";

#[derive(Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Layout {
    /// 가로
    Horizontal,
    /// 세로
    Vertical,
    /// 격자
    #[default]
    Grid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend("x-preview" = "none"))]
struct SpriteOptions {
    /// 배치
    ///
    /// 넣은 순서대로 놓습니다. 격자는 정사각형에 가깝게 열 수를 정하고, 칸 크기는 가장 큰 이미지에 맞춥니다.
    layout: Layout,
    /// 간격(px)
    ///
    /// 이미지 사이를 띄울 거리입니다. 확대·축소할 때 옆 이미지가 비쳐 보이지 않게 합니다.
    #[schemars(range(min = 0, max = 100))]
    gap: u32,
    /// 접두사
    ///
    /// CSS 클래스 이름 앞부분입니다. 공통 클래스 이름이 되고, 이미지마다 `접두사-파일이름` 클래스를 만듭니다. 예: icon → .icon, .icon-home
    prefix: String,
    /// 2x 레티나
    ///
    /// 넣은 이미지를 2배 해상도로 보고 sprite@2x.png(원래 크기)와 sprite.png(절반 크기)를 만듭니다. CSS의 크기·좌표는 절반 기준입니다.
    retina: bool,
}

impl Default for SpriteOptions {
    fn default() -> Self {
        Self {
            layout: Layout::Grid,
            gap: 2,
            prefix: "sprite".into(),
            retina: false,
        }
    }
}

struct ImageSprite;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageSprite)
}

waple_tools_addon_abi::export_tool!(tool);

/// 칸 크기들을 배치해 각 칸의 왼쪽 위 좌표와 전체 크기를 낸다. 넘치지 않게 u64로 계산한다.
fn arrange(cells: &[(u64, u64)], layout: Layout, gap: u64) -> (Vec<(u64, u64)>, u64, u64) {
    let n = cells.len() as u64;
    let max_w = cells.iter().map(|c| c.0).max().unwrap_or(0);
    let max_h = cells.iter().map(|c| c.1).max().unwrap_or(0);
    // 한 줄 배치: 앞에서부터 칸 길이 + 간격만큼 민다
    let line = |len: fn(&(u64, u64)) -> u64| {
        let mut at = 0;
        let starts: Vec<u64> = cells
            .iter()
            .map(|c| {
                let start = at;
                at += len(c) + gap;
                start
            })
            .collect();
        (starts, at.saturating_sub(gap))
    };
    match layout {
        Layout::Horizontal => {
            let (xs, w) = line(|c| c.0);
            (xs.into_iter().map(|x| (x, 0)).collect(), w, max_h)
        }
        Layout::Vertical => {
            let (ys, h) = line(|c| c.1);
            (ys.into_iter().map(|y| (0, y)).collect(), max_w, h)
        }
        Layout::Grid => {
            let cols = (1..).find(|c| c * c >= n).unwrap_or(1);
            let rows = n.div_ceil(cols);
            let pos = (0..n)
                .map(|i| (i % cols * (max_w + gap), i / cols * (max_h + gap)))
                .collect();
            (
                pos,
                (cols * (max_w + gap)).saturating_sub(gap),
                (rows * (max_h + gap)).saturating_sub(gap),
            )
        }
    }
}

/// 파일 이름 → CSS 클래스 조각: 소문자, 공백·밑줄 → `-`, 식별자에 못 쓰는 문자는 뺀다.
/// 한글 같은 유니코드 글자는 CSS 식별자에 그대로 쓸 수 있어 남긴다.
fn class_name(stem: &str) -> String {
    let mut out = String::new();
    for c in stem.chars().flat_map(char::to_lowercase) {
        if c.is_whitespace() || c == '_' || c == '-' {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else if c.is_ascii_alphanumeric() || (!c.is_ascii() && c.is_alphanumeric()) {
            out.push(c);
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "image" } else { out }.to_string()
}

/// 겹치는 이름은 뒤에 나온 것부터 `-2`, `-3`
fn unique_names(stems: &[String]) -> Vec<String> {
    let mut used = HashSet::new();
    stems
        .iter()
        .map(|s| {
            let base = class_name(s);
            let mut name = base.clone();
            for n in 2.. {
                if used.insert(name.clone()) {
                    break;
                }
                name = format!("{base}-{n}");
            }
            name
        })
        .collect()
}

fn check_prefix(p: &str) -> Result<(), ToolError> {
    let ok = p
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && p.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(ToolError::InvalidOptions(
            "클래스 접두사는 글자로 시작하고 글자·숫자·-·_만 쓸 수 있습니다.".into(),
        ))
    }
}

fn offset(v: u64) -> String {
    if v == 0 {
        "0".into()
    } else {
        format!("-{v}px")
    }
}

/// 한 이미지의 클래스 이름, 칸 크기, 좌표 (CSS 기준 = 1x)
type Item = (String, (u64, u64), (u64, u64));

/// `retina`는 1x 스프라이트 전체 크기
fn css(prefix: &str, items: &[Item], retina: Option<(u64, u64)>) -> String {
    let mut s = String::from("@charset \"UTF-8\";\n");
    let first = &items[0].0;
    let _ = writeln!(
        s,
        "/* 사용 예: <i class=\"{prefix} {prefix}-{first}\"></i> */\n"
    );
    let _ = writeln!(
        s,
        ".{prefix} {{\n  display: inline-block;\n  background-image: url(sprite.png);\n  background-repeat: no-repeat;\n}}\n"
    );
    for (name, (w, h), (x, y)) in items {
        let (x, y) = (offset(*x), offset(*y));
        let _ = writeln!(
            s,
            ".{prefix}-{name} {{\n  width: {w}px;\n  height: {h}px;\n  background-position: {x} {y};\n}}\n"
        );
    }
    if let Some((w, h)) = retina {
        let _ = writeln!(
            s,
            "@media (-webkit-min-device-pixel-ratio: 2), (min-resolution: 192dpi) {{\n  .{prefix} {{\n    background-image: url(sprite@2x.png);\n    background-size: {w}px {h}px;\n  }}\n}}"
        );
    }
    s
}

fn write_png(out_dir: &Path, file: &str, img: RgbaImage) -> Result<PathBuf, ToolError> {
    let bytes = encode(
        &DynamicImage::ImageRgba8(img),
        Fmt::Png,
        Quality {
            quality: 100,
            lossless: true,
        },
    )?;
    let rel = Path::new(DIR).join(file);
    std::fs::write(out_dir.join(&rel), bytes)?;
    Ok(rel)
}

impl Tool for ImageSprite {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.sprite".into(),
            name: "스프라이트 생성".into(),
            category: "image".into(),
            description: "여러 이미지를 한 장으로 합치고 CSS를 만듭니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Combine
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<SpriteOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: SpriteOptions = parse_options(opts)?;
        let prefix = o.prefix.trim();
        check_prefix(prefix)?;
        if input.is_empty() {
            return Err(ToolError::Failed("합칠 이미지를 넣어 주세요.".into()));
        }
        // 2x는 칸을 짝수로 맞춰 절반으로 줄여도 좌표가 정수로 떨어지게 한다
        let scale: u32 = if o.retina { 2 } else { 1 };
        let even = |v: u32| u64::from(v).next_multiple_of(u64::from(scale));
        // 결과는 적어도 칸 넓이의 합이다: 다 디코딩하기 전에 머리글의 크기만 읽어 먼저 막는다.
        // 크기를 못 읽는 파일은 아래 load가 이름과 함께 알린다
        let mut total: u64 = 0;
        for p in input {
            if let Ok((w, h)) = image::image_dimensions(p) {
                total += even(w) * even(h);
                ensure_pixels(u32::try_from(total).unwrap_or(u32::MAX), 1)?;
            }
        }
        let mut images = Vec::with_capacity(input.len());
        let mut stems = Vec::with_capacity(input.len());
        for (i, p) in input.iter().enumerate() {
            ctx.check_cancel()?;
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            let img = load(p).map_err(|e| ToolError::Failed(format!("{name}: {e}")))?;
            images.push(img.image.to_rgba8());
            stems.push(
                p.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            );
            ctx.progress((i + 1) as f32 / (input.len() + 1) as f32);
        }

        let cells: Vec<(u64, u64)> = images
            .iter()
            .map(|im| (even(im.width()), even(im.height())))
            .collect();
        let gap = u64::from(o.gap.min(100) * scale);
        let (pos, w, h) = arrange(&cells, o.layout, gap);
        let fit = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
        ensure_pixels(fit(w), fit(h))?;
        let (w, h) = (w as u32, h as u32);

        let mut full = RgbaImage::new(w, h);
        for (im, &(x, y)) in images.iter().zip(&pos) {
            imageops::replace(&mut full, im, x as i64, y as i64);
        }
        let s = u64::from(scale);
        let items: Vec<Item> = unique_names(&stems)
            .into_iter()
            .zip(cells.iter().zip(&pos))
            .map(|(n, (&(cw, ch), &(x, y)))| (n, (cw / s, ch / s), (x / s, y / s)))
            .collect();

        std::fs::create_dir_all(out_dir.join(DIR))?;
        let mut names = Vec::new();
        let size_1x = if o.retina {
            // 칸마다 따로 줄여야 옆 이미지가 섞이지 않는다
            let mut half = RgbaImage::new(w / 2, h / 2);
            for (im, (_, (cw, ch), (x, y))) in images.iter().zip(&items) {
                let (cw, ch) = (*cw as u32, *ch as u32);
                let mut cell = RgbaImage::new(cw * 2, ch * 2);
                imageops::replace(&mut cell, im, 0, 0);
                let small = resize_rgba(&cell, cw, ch, FilterType::Lanczos3);
                imageops::replace(&mut half, &small, *x as i64, *y as i64);
            }
            names.push(write_png(out_dir, "sprite.png", half)?);
            names.push(write_png(out_dir, "sprite@2x.png", full)?);
            Some((u64::from(w / 2), u64::from(h / 2)))
        } else {
            names.push(write_png(out_dir, "sprite.png", full)?);
            None
        };
        let rel = Path::new(DIR).join("sprite.css");
        std::fs::write(out_dir.join(&rel), css(prefix, &items, size_1x))?;
        names.push(rel);
        ctx.progress(1.0);
        Ok(ToolOutput::Files(names))
    }
}
