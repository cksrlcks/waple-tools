//! 아이콘 생성 (추가 설치, Generator): 이미지 한 장 → 웹 파비콘 세트 · 앱 아이콘 세트 (`<이름>-icons/` 폴더)

mod pack;
mod render;
mod sets;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};
use waple_tools_image_common::{INPUT_EXTS, load, parse_color, parse_options, schema_of};

use crate::render::square_master;
use crate::sets::{Files, Painter};

#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[schemars(extend(
    "x-require-any" = ["favicon", "android", "ios", "windows", "macos"],
    // 결과는 크기·모양이 원본과 달라 겹쳐 비교하지 않는다
    "x-preview" = "result"
))]
struct IconOptions {
    /// 웹 파비콘
    ///
    /// favicon.ico, PNG 여러 크기, apple-touch-icon, site.webmanifest, `<head>`에 넣을 코드(head.html)
    #[schemars(extend("x-group" = "만들 세트"))]
    favicon: bool,
    /// Android
    ///
    /// mipmap 밀도별 아이콘·둥근 아이콘·적응형 아이콘, Play 스토어용 512px
    #[schemars(extend("x-group" = "만들 세트"))]
    android: bool,
    /// iOS
    ///
    /// AppIcon.appiconset 폴더(전체 크기 + Contents.json). 투명 부분은 배경색으로 채웁니다.
    #[schemars(extend("x-group" = "만들 세트"))]
    ios: bool,
    /// Windows (.ico)
    #[schemars(extend("x-group" = "만들 세트"))]
    windows: bool,
    /// macOS (.icns)
    #[schemars(extend("x-group" = "만들 세트"))]
    macos: bool,

    /// 여백(%)
    ///
    /// 가장자리에서 안쪽으로 띄울 거리입니다 (한 변 기준).
    #[schemars(range(min = 0, max = 40))]
    #[schemars(extend("x-group" = "모양"))]
    padding: u8,
    /// 둥근 모서리(%)
    ///
    /// 50이면 원입니다. iOS와 Android 적응형 아이콘은 OS가 모양을 깎으므로 적용하지 않습니다.
    #[schemars(range(min = 0, max = 50))]
    #[schemars(extend("x-group" = "모양"))]
    radius: u8,
    /// 배경 채우기
    ///
    /// 끄면 투명 배경입니다. 투명을 쓸 수 없는 iOS·apple-touch-icon·Play 스토어 아이콘은 흰색으로 채웁니다.
    #[schemars(extend("x-group" = "모양"))]
    fill: bool,
    /// 배경색
    #[schemars(extend("format" = "color", "x-group" = "모양", "x-enabled-when" = { "fill": [true] }))]
    background: String,
}

impl Default for IconOptions {
    fn default() -> Self {
        Self {
            favicon: true,
            android: false,
            ios: false,
            windows: false,
            macos: false,
            padding: 0,
            radius: 0,
            fill: false,
            background: "#ffffff".into(),
        }
    }
}

struct ImageIcon;

pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageIcon)
}

waple_tools_addon_abi::export_tool!(tool);

/// 결과 세트를 만든다 (상대 경로, 내용). 첫 항목은 미리보기용 큰 PNG.
fn build(
    img: &image::DynamicImage,
    name: &str,
    o: &IconOptions,
    ctx: &JobContext,
) -> Result<Files, ToolError> {
    let background = if o.fill {
        Some(parse_color(&o.background).ok_or_else(|| {
            ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.".into())
        })?)
    } else {
        None
    };
    let master = square_master(img);
    let mut p = Painter::new(
        &master,
        f32::from(o.padding.min(40)) / 100.0,
        f32::from(o.radius.min(50)) / 100.0,
        background,
    );
    let mut files = Files::new();
    type Step<'p, 'a> = fn(&mut Painter<'a>, &str, &mut Files) -> Result<(), ToolError>;
    let steps: [(bool, Step); 5] = [
        (o.favicon, |p, n, f| p.favicon(n, f)),
        (o.android, |p, _, f| p.android(f)),
        (o.ios, |p, _, f| p.ios(f)),
        (o.windows, |p, _, f| p.windows(f)),
        (o.macos, |p, _, f| p.macos(f)),
    ];
    let chosen: Vec<_> = steps.into_iter().filter(|(on, _)| *on).collect();
    if chosen.is_empty() {
        return Err(ToolError::InvalidOptions(
            "만들 세트를 하나 이상 골라 주세요.".into(),
        ));
    }
    for (i, (_, step)) in chosen.iter().enumerate() {
        ctx.check_cancel()?;
        step(&mut p, name, &mut files)?;
        ctx.progress((i + 1) as f32 / chosen.len() as f32);
    }
    // 미리보기는 첫 결과를 보여 준다: 가장 큰 PNG를 앞으로
    if let Some(i) = (0..files.len())
        .filter(|&i| files[i].0.ends_with(".png"))
        .max_by_key(|&i| files[i].1.len())
    {
        files.swap(0, i);
    }
    Ok(files)
}

impl Tool for ImageIcon {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image.icon".into(),
            name: "아이콘 생성".into(),
            category: "image".into(),
            description: "웹 파비콘과 앱 아이콘 세트를 만듭니다.".into(),
            extensions: INPUT_EXTS.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Generator
    }

    fn options_schema(&self) -> serde_json::Value {
        schema_of::<IconOptions>()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let o: IconOptions = parse_options(opts)?;
        let src = input
            .first()
            .ok_or_else(|| ToolError::Failed("이미지를 넣어 주세요.".into()))?;
        let stem = src
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let img = load(src)?.image;
        let files = build(&img, &stem, &o, ctx)?;
        let top = format!("{stem}-icons");
        let mut names = Vec::with_capacity(files.len());
        for (rel, bytes) in files {
            let name = PathBuf::from(&top).join(rel);
            let path = out_dir.join(&name);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, bytes)?;
            names.push(name);
        }
        Ok(ToolOutput::Files(names))
    }
}
