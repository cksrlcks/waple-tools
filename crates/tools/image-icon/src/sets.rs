//! 대상별 결과 세트 (docs/plan.md 4장): 웹 파비콘 · Android · iOS · Windows · macOS

use std::collections::HashMap;

use image::{DynamicImage, RgbaImage};
use waple_tools_core::ToolError;
use waple_tools_image_common::{Fmt, Quality, encode};

use crate::pack::{ICNS_TYPES, icns, ico};
use crate::render::{Mask, Style, render};

/// 결과 세트 안 상대 경로와 내용
pub type Files = Vec<(String, Vec<u8>)>;

pub struct Painter<'a> {
    master: &'a RgbaImage,
    padding: f32,
    radius: f32,
    background: Option<[u8; 3]>,
    cache: HashMap<String, Vec<u8>>,
}

/// 투명을 못 쓰는 곳(iOS, apple-touch-icon, Play 스토어)의 배경: 고른 색, 없으면 흰색
const WHITE: [u8; 3] = [255, 255, 255];

impl<'a> Painter<'a> {
    pub fn new(
        master: &'a RgbaImage,
        padding: f32,
        radius: f32,
        background: Option<[u8; 3]>,
    ) -> Self {
        Self {
            master,
            padding,
            radius,
            background,
            cache: HashMap::new(),
        }
    }

    fn opaque_bg(&self) -> [u8; 3] {
        self.background.unwrap_or(WHITE)
    }

    /// 옵션 그대로: 여백 · 둥근 모서리 · 배경
    fn plain(&self) -> Style {
        Style {
            padding: self.padding,
            safe: 1.0,
            mask: Mask::Rounded(self.radius),
            background: self.background,
        }
    }

    /// 투명 없이 꽉 찬 정사각형 (모서리는 OS가 깎는다)
    fn opaque(&self) -> Style {
        Style {
            mask: Mask::None,
            background: Some(self.opaque_bg()),
            ..self.plain()
        }
    }

    fn round(&self) -> Style {
        Style {
            mask: Mask::Circle,
            ..self.plain()
        }
    }

    /// Android 적응형 아이콘 전경: 108dp 중 가운데 66dp 안전 영역에만 그린다. 배경은 따로 색으로
    fn foreground(&self) -> Style {
        Style {
            safe: 66.0 / 108.0,
            mask: Mask::None,
            background: None,
            ..self.plain()
        }
    }

    fn png(&mut self, n: u32, s: Style) -> Result<Vec<u8>, ToolError> {
        let key = format!("{n}:{s:?}");
        if let Some(b) = self.cache.get(&key) {
            return Ok(b.clone());
        }
        let img = DynamicImage::ImageRgba8(render(self.master, n, s));
        let bytes = encode(
            &img,
            Fmt::Png,
            Quality {
                quality: 100,
                lossless: true,
            },
        )?;
        self.cache.insert(key, bytes.clone());
        Ok(bytes)
    }

    fn ico(&mut self, sizes: &[u32]) -> Result<Vec<u8>, ToolError> {
        let s = self.plain();
        let images = sizes
            .iter()
            .map(|&n| self.png(n, s).map(|b| (n, b)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ico(&images))
    }

    pub fn favicon(&mut self, name: &str, out: &mut Files) -> Result<(), ToolError> {
        let (plain, opaque) = (self.plain(), self.opaque());
        out.push(("favicon/favicon.ico".into(), self.ico(&[16, 32, 48])?));
        for n in [16, 32] {
            out.push((format!("favicon/favicon-{n}x{n}.png"), self.png(n, plain)?));
        }
        out.push((
            "favicon/apple-touch-icon.png".into(),
            self.png(180, opaque)?,
        ));
        for n in [192, 512] {
            out.push((
                format!("favicon/android-chrome-{n}x{n}.png"),
                self.png(n, plain)?,
            ));
        }
        let bg = hex(self.opaque_bg());
        let manifest = serde_json::json!({
            "name": name,
            "short_name": name,
            "icons": [
                { "src": "/android-chrome-192x192.png", "sizes": "192x192", "type": "image/png" },
                { "src": "/android-chrome-512x512.png", "sizes": "512x512", "type": "image/png" }
            ],
            "theme_color": bg,
            "background_color": bg,
            "display": "standalone"
        });
        out.push((
            "favicon/site.webmanifest".into(),
            serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
        ));
        out.push(("favicon/head.html".into(), HEAD_HTML.as_bytes().to_vec()));
        Ok(())
    }

    pub fn android(&mut self, out: &mut Files) -> Result<(), ToolError> {
        let (plain, round, fg) = (self.plain(), self.round(), self.foreground());
        for (density, n) in ANDROID_DENSITIES {
            let dir = format!("android/res/mipmap-{density}");
            out.push((format!("{dir}/ic_launcher.png"), self.png(n, plain)?));
            out.push((format!("{dir}/ic_launcher_round.png"), self.png(n, round)?));
            // 적응형 아이콘은 48dp가 아니라 108dp 기준
            out.push((
                format!("{dir}/ic_launcher_foreground.png"),
                self.png(n * 108 / 48, fg)?,
            ));
        }
        for name in ["ic_launcher", "ic_launcher_round"] {
            out.push((
                format!("android/res/mipmap-anydpi-v26/{name}.xml"),
                ADAPTIVE_XML.as_bytes().to_vec(),
            ));
        }
        out.push((
            "android/res/values/ic_launcher_background.xml".into(),
            format!(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <color name=\"ic_launcher_background\">{}</color>\n</resources>\n",
                hex(self.opaque_bg()).to_uppercase()
            )
            .into_bytes(),
        ));
        let opaque = self.opaque();
        out.push(("android/play-store-512.png".into(), self.png(512, opaque)?));
        Ok(())
    }

    pub fn ios(&mut self, out: &mut Files) -> Result<(), ToolError> {
        let opaque = self.opaque();
        let mut images = vec![];
        let mut done = std::collections::BTreeSet::new();
        for &(idiom, pt, scale) in IOS_ICONS {
            let px = (pt * scale as f32).round() as u32;
            let file = format!("Icon-{px}.png");
            if done.insert(px) {
                out.push((
                    format!("ios/AppIcon.appiconset/{file}"),
                    self.png(px, opaque)?,
                ));
            }
            let pt_s = if pt.fract() == 0.0 {
                format!("{pt:.0}")
            } else {
                format!("{pt}")
            };
            images.push(serde_json::json!({
                "size": format!("{pt_s}x{pt_s}"),
                "idiom": idiom,
                "filename": file,
                "scale": format!("{scale}x"),
            }));
        }
        let contents = serde_json::json!({
            "images": images,
            "info": { "version": 1, "author": "waple tools" }
        });
        out.push((
            "ios/AppIcon.appiconset/Contents.json".into(),
            serde_json::to_vec_pretty(&contents).unwrap_or_default(),
        ));
        Ok(())
    }

    pub fn windows(&mut self, out: &mut Files) -> Result<(), ToolError> {
        out.push((
            "windows/app.ico".into(),
            self.ico(&[16, 24, 32, 48, 64, 128, 256])?,
        ));
        Ok(())
    }

    pub fn macos(&mut self, out: &mut Files) -> Result<(), ToolError> {
        let s = self.plain();
        let pngs = ICNS_TYPES
            .iter()
            .map(|&(code, n)| self.png(n, s).map(|b| (code, b)))
            .collect::<Result<Vec<_>, _>>()?;
        let entries: Vec<_> = pngs.iter().map(|(c, b)| (*c, b.as_slice())).collect();
        out.push(("macos/app.icns".into(), icns(&entries)));
        Ok(())
    }
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

const ANDROID_DENSITIES: [(&str, u32); 5] = [
    ("mdpi", 48),
    ("hdpi", 72),
    ("xhdpi", 96),
    ("xxhdpi", 144),
    ("xxxhdpi", 192),
];

/// (idiom, 포인트 크기, 배율) — Xcode AppIcon 전체 목록
const IOS_ICONS: &[(&str, f32, u32)] = &[
    ("iphone", 20.0, 2),
    ("iphone", 20.0, 3),
    ("iphone", 29.0, 2),
    ("iphone", 29.0, 3),
    ("iphone", 40.0, 2),
    ("iphone", 40.0, 3),
    ("iphone", 60.0, 2),
    ("iphone", 60.0, 3),
    ("ipad", 20.0, 1),
    ("ipad", 20.0, 2),
    ("ipad", 29.0, 1),
    ("ipad", 29.0, 2),
    ("ipad", 40.0, 1),
    ("ipad", 40.0, 2),
    ("ipad", 76.0, 1),
    ("ipad", 76.0, 2),
    ("ipad", 83.5, 2),
    ("ios-marketing", 1024.0, 1),
];

const ADAPTIVE_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
</adaptive-icon>
"#;

/// favicon 폴더 파일을 사이트 루트에 두고 `<head>`에 넣는 코드
const HEAD_HTML: &str = r#"<link rel="icon" href="/favicon.ico" sizes="48x48">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
"#;
