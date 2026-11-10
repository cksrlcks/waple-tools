use std::path::Path;

/// 이 앱이 읽고 쓰는 이미지 포맷
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fmt {
    Jpeg,
    Png,
    Webp,
    Gif,
    Bmp,
    Tiff,
}

/// 기본 내장 이미지 도구가 받는 확장자
pub const INPUT_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"];

impl Fmt {
    pub fn from_path(p: &Path) -> Option<Fmt> {
        let ext = p.extension()?.to_string_lossy().to_lowercase();
        Some(match ext.as_str() {
            "jpg" | "jpeg" => Fmt::Jpeg,
            "png" => Fmt::Png,
            "webp" => Fmt::Webp,
            "gif" => Fmt::Gif,
            "bmp" => Fmt::Bmp,
            "tif" | "tiff" => Fmt::Tiff,
            _ => return None,
        })
    }

    pub fn ext(self) -> &'static str {
        match self {
            Fmt::Jpeg => "jpg",
            Fmt::Png => "png",
            Fmt::Webp => "webp",
            Fmt::Gif => "gif",
            Fmt::Bmp => "bmp",
            Fmt::Tiff => "tiff",
        }
    }

    pub(crate) fn from_image(f: image::ImageFormat) -> Option<Fmt> {
        use image::ImageFormat as F;
        Some(match f {
            F::Jpeg => Fmt::Jpeg,
            F::Png => Fmt::Png,
            F::WebP => Fmt::Webp,
            F::Gif => Fmt::Gif,
            F::Bmp => Fmt::Bmp,
            F::Tiff => Fmt::Tiff,
            _ => return None,
        })
    }

    pub(crate) fn to_image(self) -> image::ImageFormat {
        use image::ImageFormat as F;
        match self {
            Fmt::Jpeg => F::Jpeg,
            Fmt::Png => F::Png,
            Fmt::Webp => F::WebP,
            Fmt::Gif => F::Gif,
            Fmt::Bmp => F::Bmp,
            Fmt::Tiff => F::Tiff,
        }
    }
}
