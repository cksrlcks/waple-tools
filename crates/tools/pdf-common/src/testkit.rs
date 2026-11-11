//! 테스트용 PDF 만들기·읽기. 페이지마다 `(Page N)` 글자와 색 사각형이 있다.

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};

/// `pages`쪽 A4 PDF. 글꼴·크기는 페이지 트리 루트에서 물려받는다 (상속 처리 확인용).
/// `label`이 페이지 글자 앞에 붙는다 (`"a"` → `(a Page 1)`)
pub fn sample_pdf(pages: u32, label: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let mut kids = vec![];
    for i in 1..=pages {
        let text = if label.is_empty() {
            format!("Page {i}")
        } else {
            format!("{label} Page {i}")
        };
        let content = Content {
            operations: vec![
                Operation::new("rg", vec![0.2.into(), 0.4.into(), 0.8.into()]),
                Operation::new("re", vec![50.into(), 50.into(), 200.into(), 200.into()]),
                Operation::new("f", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 36.into()]),
                Operation::new("Td", vec![100.into(), 600.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        kids.push(
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
            })
            .into(),
        );
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => pages,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let mut out = vec![];
    doc.save_to(&mut out).unwrap();
    out
}

/// A4 한 쪽에 `w`×`h` RGB 사진(Flate, 잡음 섞인 그라데이션)이 페이지 전체를 덮는 PDF
pub fn image_pdf(w: u32, h: u32) -> Vec<u8> {
    let mut seed = 0x2545_f491_u32;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let n = (seed % 24) as u8;
            rgb.extend([(x * 200 / w) as u8 + n, (y * 200 / h) as u8 + n, 120 + n]);
        }
    }
    let mut doc = Document::with_version("1.5");
    let mut img = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => w,
            "Height" => h,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
        },
        rgb,
    );
    img.compress().unwrap();
    let img_id = doc.add_object(img);
    let content = b"q 595 0 0 842 0 0 cm /Im1 Do Q".to_vec();
    let content_id = doc.add_object(Stream::new(dictionary! {}, content));
    let pages_id = doc.new_object_id();
    let page = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Contents" => content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Im1" => img_id } },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut out = vec![];
    doc.save_to(&mut out).unwrap();
    out
}

/// 문서 안 이미지들의 (가로, 세로, 필터 이름)
pub fn images(bytes: &[u8]) -> Vec<(i64, i64, String)> {
    let doc = Document::load_mem(bytes).unwrap();
    doc.objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| s.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
        .map(|s| {
            let n = |k: &[u8]| s.dict.get(k).and_then(Object::as_i64).unwrap_or(0);
            let filter = s
                .dict
                .get(b"Filter")
                .and_then(Object::as_name)
                .map(|f| String::from_utf8_lossy(f).into_owned())
                .unwrap_or_default();
            (n(b"Width"), n(b"Height"), filter)
        })
        .collect()
}

/// 페이지마다 `Tj`로 쓴 글자 (순서 확인용)
pub fn page_labels(bytes: &[u8]) -> Vec<String> {
    let doc = Document::load_mem(bytes).unwrap();
    doc.get_pages()
        .values()
        .map(|&id| {
            let content = Content::decode(&doc.get_page_content(id)).unwrap();
            content
                .operations
                .iter()
                .filter(|op| op.operator == "Tj")
                .filter_map(|op| op.operands.first())
                .filter_map(|o| o.as_str().ok())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// 결과 비교용 요약: PDF는 (페이지 글자, 이미지 목록), 그 밖은 바이트 그대로.
/// DLL과 내장 빌드는 압축 라이브러리 기능이 달라 Flate 바이트가 다를 수 있다.
pub fn summary(name: &std::path::Path, bytes: &[u8]) -> String {
    if name.extension().is_some_and(|e| e == "pdf") {
        format!("{:?} {:?}", page_labels(bytes), images(bytes))
    } else {
        format!("{bytes:?}")
    }
}

pub fn page_count(bytes: &[u8]) -> usize {
    Document::load_mem(bytes).unwrap().get_pages().len()
}
