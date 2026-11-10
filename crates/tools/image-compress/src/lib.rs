//! 이미지 압축 (기본 내장): 포맷 유지. JPG=mozjpeg, WebP=재인코딩, PNG=oxipng(+선택적 색상 수 줄이기)

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, RgbaImage};
use localtools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use localtools_image_common::{
    Fmt, MetaMode, Quality, encode, load, optimize_png, parse_options, run_each, schema_of,
    with_meta, write_output,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Mode {
    /// 품질 지정
    #[default]
    Quality,
    /// 목표 용량
    TargetKb,
}

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
struct CompressOptions {
    /// 방식 (JPG·WebP)
    mode: Mode,
    /// 품질
    #[schemars(range(min = 1, max = 100))]
    quality: u8,
    /// 목표 용량(KB)
    #[schemars(range(min = 1))]
    target_kb: u32,
    /// PNG 색상 수 (0은 줄이지 않음, 손실)
    #[schemars(range(min = 0, max = 256))]
    png_colors: u16,
    /// 메타데이터
    metadata: MetaMode,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Quality,
            quality: 75,
            target_kb: 200,
            png_colors: 0,
            metadata: MetaMode::Strip,
        }
    }
}

/// 품질 1~100 이진 탐색: `target` 바이트 이하인 가장 높은 품질의 결과.
/// 품질 1로도 크면 품질 1 결과(가장 작은 결과)를 돌려준다. 인코딩은 최대 8번.
fn search_quality(
    target: usize,
    encode: impl Fn(u8) -> Result<Vec<u8>, ToolError>,
) -> Result<Vec<u8>, ToolError> {
    let (mut lo, mut hi) = (1u8, 100u8);
    let mut best: Option<Vec<u8>> = None;
    let mut at_one: Option<Vec<u8>> = None;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let bytes = encode(mid)?;
        if bytes.len() <= target {
            best = Some(bytes);
            lo = mid + 1;
        } else {
            if mid == 1 {
                at_one = Some(bytes);
                break;
            }
            hi = mid - 1;
        }
    }
    match (best, at_one) {
        (Some(b), _) | (None, Some(b)) => Ok(b),
        (None, None) => encode(1),
    }
}

/// PNG 색상 수 줄이기. 완전 투명/불투명만 있으면 불투명 픽셀의 색만 줄이고,
/// 반투명 픽셀이 있으면 줄이지 않는다(그대로 돌려줌). oxipng가 256색 이하를 팔레트 PNG로 바꾼다.
fn reduce_colors(img: &DynamicImage, colors: u16) -> DynamicImage {
    let rgba = img.to_rgba8();
    if rgba.pixels().any(|p| p.0[3] != 0 && p.0[3] != 255) {
        return img.clone();
    }
    let has_transparent = rgba.pixels().any(|p| p.0[3] == 0);
    // 투명 칸 하나를 남겨 둔다
    let size = if has_transparent { colors - 1 } else { colors }.clamp(2, 256);
    let Ok(palette_size) = size.try_into() else {
        return img.clone();
    };
    let Ok(buf) = quantette::ImageBuf::try_from(img.to_rgb8()) else {
        return img.clone();
    };
    let quantized = quantette::Pipeline::new()
        .palette_size(palette_size)
        .input_image(buf.as_ref())
        .output_srgb8_image();
    let colors: Vec<[u8; 3]> = quantized
        .as_slice()
        .iter()
        .map(|c| [c.red, c.green, c.blue])
        .collect();
    let out = RgbaImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        if rgba.get_pixel(x, y).0[3] == 0 {
            image::Rgba([0, 0, 0, 0])
        } else {
            let [r, g, b] = colors[(y * rgba.width() + x) as usize];
            image::Rgba([r, g, b, 255])
        }
    });
    DynamicImage::ImageRgba8(out)
}

fn compress_png(img: &DynamicImage, colors: u16) -> Result<Vec<u8>, ToolError> {
    let img = if colors >= 2 {
        reduce_colors(img, colors)
    } else {
        img.clone()
    };
    let mut raw = Vec::new();
    img.write_to(&mut Cursor::new(&mut raw), image::ImageFormat::Png)
        .map_err(|_| ToolError::Failed("이미지를 저장할 수 없습니다.".into()))?;
    optimize_png(&raw, 4)
}

struct Compress;

pub fn tool() -> Box<dyn Tool> {
    Box::new(Compress)
}

fn compress(input: &Path, out_dir: &Path, o: &CompressOptions) -> Result<PathBuf, ToolError> {
    let src = load(input)?;
    let fmt = src.format;
    let lossy = |q: u8| {
        encode(
            &src.image,
            fmt,
            Quality {
                quality: q,
                lossless: false,
            },
        )
    };
    let mut bytes = match fmt {
        Fmt::Jpeg | Fmt::Webp => match o.mode {
            Mode::Quality => lossy(o.quality.clamp(1, 100))?,
            Mode::TargetKb => search_quality(o.target_kb.max(1) as usize * 1024, lossy)?,
        },
        Fmt::Png => compress_png(&src.image, o.png_colors.min(256))?,
        Fmt::Gif | Fmt::Bmp | Fmt::Tiff => {
            return Err(ToolError::Failed(
                "이 형식은 압축을 지원하지 않습니다.".into(),
            ));
        }
    };
    if o.metadata == MetaMode::Keep {
        bytes = with_meta(bytes, fmt, &src.meta)?;
    }
    write_output(out_dir, input, fmt, &bytes)
}

impl Tool for Compress {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.compress".into(),
            name: "이미지 압축".into(),
            category: "image".into(),
            description: "품질 또는 목표 용량으로 같은 포맷 그대로 압축합니다.".into(),
            extensions: ["jpg", "jpeg", "png", "webp"].map(String::from).to_vec(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<CompressOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: CompressOptions = parse_options(opts)?;
        run_each(ctx, input, |p| compress(p, out_dir, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::{JobContext, ToolError, ToolOutput};
    use localtools_image_common::testkit::{alpha_png, photo};
    use localtools_image_common::{Fmt, Quality, encode};
    use serde_json::json;
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    fn run(input: &Path, out: &Path, v: serde_json::Value) -> Result<PathBuf, ToolError> {
        let ctx = JobContext::new(|_| {}, Default::default());
        match tool().run(&ctx, &[input.to_path_buf()], out, &v)? {
            ToolOutput::Files(f) => Ok(f[0].clone()),
            other => panic!("{other:?}"),
        }
    }

    fn jpeg_at(img: &image::DynamicImage, q: u8) -> Result<Vec<u8>, ToolError> {
        encode(
            img,
            Fmt::Jpeg,
            Quality {
                quality: q,
                lossless: false,
            },
        )
    }

    #[test]
    fn search_quality_hits_target_within_5_percent() {
        let img = photo(1024, 768);
        let lo = jpeg_at(&img, 40).unwrap().len();
        let hi = jpeg_at(&img, 80).unwrap().len();
        let target = (lo + hi) / 2;
        let out = search_quality(target, |q| jpeg_at(&img, q)).unwrap();
        assert!(out.len() <= target, "{} > {target}", out.len());
        let gap = (target - out.len()) as f64 / target as f64;
        assert!(gap <= 0.05, "목표와 {:.1}% 차이", gap * 100.0);
    }

    #[test]
    fn search_quality_encodes_at_most_8_times() {
        let img = photo(256, 256);
        let calls = Cell::new(0);
        let target = jpeg_at(&img, 55).unwrap().len();
        search_quality(target, |q| {
            calls.set(calls.get() + 1);
            jpeg_at(&img, q)
        })
        .unwrap();
        assert!(calls.get() <= 8, "{}번", calls.get());
    }

    #[test]
    fn target_kb_unreachable_returns_smallest() {
        let img = photo(128, 128);
        let out = search_quality(1, |q| jpeg_at(&img, q)).unwrap();
        assert_eq!(out.len(), jpeg_at(&img, 1).unwrap().len());
    }

    #[test]
    fn quality_mode_shrinks_jpeg() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.jpg");
        std::fs::write(&src, jpeg_at(&photo(512, 512), 98).unwrap()).unwrap();
        // 엔진처럼 출력 폴더를 따로 둔다 (같은 폴더면 결과가 원본을 덮어씀)
        let out = d.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let name = run(&src, &out, json!({ "quality": 60 })).unwrap();
        assert_eq!(name, Path::new("a.jpg"));
        assert!(
            std::fs::metadata(out.join(&name)).unwrap().len()
                < std::fs::metadata(&src).unwrap().len()
        );
    }

    #[test]
    fn png_is_lossless() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        let img = photo(64, 64);
        img.save(&src).unwrap();
        let name = run(&src, d.path(), json!({})).unwrap();
        let back = image::open(d.path().join(name)).unwrap().to_rgb8();
        assert_eq!(back, img.to_rgb8());
    }

    fn distinct_colors(p: &Path) -> usize {
        let img = image::open(p).unwrap().to_rgba8();
        img.pixels().map(|p| p.0).collect::<HashSet<_>>().len()
    }

    #[test]
    fn png_colors_reduces_palette() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        photo(128, 128).save(&src).unwrap();
        let out_dir = d.path().join("q");
        std::fs::create_dir(&out_dir).unwrap();
        let name = run(&src, &out_dir, json!({ "pngColors": 16 })).unwrap();
        assert!(distinct_colors(&out_dir.join(&name)) <= 16);
        let lossless = d.path().join("l");
        std::fs::create_dir(&lossless).unwrap();
        run(&src, &lossless, json!({})).unwrap();
        let reduced = std::fs::metadata(out_dir.join(&name)).unwrap().len();
        let full = std::fs::metadata(lossless.join(&name)).unwrap().len();
        assert!(reduced < full, "{reduced} >= {full}");
    }

    #[test]
    fn png_colors_keeps_binary_transparency() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.png");
        alpha_png(&src, 32, 32);
        let out = d.path().join("o");
        std::fs::create_dir(&out).unwrap();
        let name = run(&src, &out, json!({ "pngColors": 8 })).unwrap();
        let img = image::open(out.join(name)).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0[3], 0, "투명 유지");
        assert_eq!(img.get_pixel(31, 0).0[3], 255, "불투명 유지");
    }

    #[test]
    fn gif_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("a.gif");
        photo(8, 8).to_rgba8().save(&src).unwrap();
        let r = run(&src, d.path(), json!({}));
        assert!(
            matches!(r, Err(ToolError::Failed(m)) if m == "이 형식은 압축을 지원하지 않습니다.")
        );
        assert_eq!(tool().meta().name, "이미지 압축");
    }
}
