use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{JobContext, ToolError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolKind {
    PerFile,
    Combine,
    Generator,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMeta {
    /// `카테고리.이름` (예: `image.webp`)
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// 소문자, 점 없음 (`"png"`). 빈 목록이면 모든 확장자.
    pub extensions: Vec<String>,
}

/// Interactive 도구의 결과 블록. 앱은 블록 종류별 뷰어만 갖는다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Text {
        text: String,
    },
    Code {
        lang: String,
        code: String,
        /// 있으면 [저장] 버튼을 보여 주고 이 이름을 기본 파일 이름으로 쓴다 (예: `qr.svg`)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Number {
        label: String,
        value: f64,
        unit: Option<String>,
    },
    /// 색 칩. 선택 필드는 없으면 JSON에서 빠져 예전 앱·도구와 호환된다
    Color {
        hex: String,
        /// 칩 이름 (예: `Vibrant`, `32.4%`)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// 칩 위에 쓸 글자색 hex (예: 본문 글자색)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// 보조 설명 (예: `rgb(…) · hsl(…)`)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// 이미지 + [이미지 복사]·[저장]. 결과가 작아야 한다 (DLL 경계를 JSON으로 넘긴다)
    Image {
        /// `data:image/png;base64,…`
        src: String,
        /// 저장할 때 기본 파일 이름 (예: `qr.png`)
        name: String,
    },
    ImagePair {
        a: PathBuf,
        b: PathBuf,
        diff: PathBuf,
    },
}

/// 추가 설치 도구는 DLL 경계를 JSON으로 넘긴다 (`{"files": [...]}` / `{"blocks": [...]}`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolOutput {
    /// `out_dir` 기준 상대 경로
    Files(Vec<PathBuf>),
    Blocks(Vec<Block>),
}

/// 모든 도구(기본 내장·추가 설치)가 따르는 계약.
/// 도구는 `out_dir`에만 쓴다. 원본 이동·백업은 배치 엔진이 한다.
pub trait Tool: Send + Sync {
    fn meta(&self) -> ToolMeta;
    fn kind(&self) -> ToolKind;
    fn options_schema(&self) -> serde_json::Value;
    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_is_tagged_camel_case() {
        let v = serde_json::to_value(Block::ImagePair {
            a: PathBuf::from("a.png"),
            b: PathBuf::from("b.png"),
            diff: PathBuf::from("d.png"),
        })
        .unwrap();
        assert_eq!(v["type"], "imagePair");
        assert_eq!(v["diff"], "d.png");
    }

    #[test]
    fn code_name_is_optional_on_the_wire() {
        let v = serde_json::to_value(Block::Code {
            lang: "css".into(),
            code: "a{}".into(),
            name: None,
        })
        .unwrap();
        assert!(v.get("name").is_none(), "예전 앱·도구와 같은 JSON");
        let b: Block = serde_json::from_value(serde_json::json!({
            "type": "code", "lang": "css", "code": "a{}"
        }))
        .unwrap();
        assert!(matches!(b, Block::Code { name: None, .. }));
    }

    #[test]
    fn color_extras_are_optional_on_the_wire() {
        let b: Block =
            serde_json::from_value(serde_json::json!({ "type": "color", "hex": "#ff0000" }))
                .unwrap();
        assert!(matches!(
            &b,
            Block::Color {
                label: None,
                text: None,
                detail: None,
                ..
            }
        ));
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "color", "hex": "#ff0000" }));
    }

    #[test]
    fn kind_is_camel_case() {
        assert_eq!(serde_json::to_value(ToolKind::PerFile).unwrap(), "perFile");
    }
}
