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
    /// 파일명 일괄 변경: `run`은 입력 순서대로 새 파일 이름(경로 없이 이름만)을 `ToolOutput::Files`로 돌려준다.
    /// 실제 이름 변경·충돌 확인·기록은 배치 엔진(`batch::rename`)이 한다. 도구는 파일을 건드리지 않는다
    Rename,
}

/// 글 조각. `mark`가 있으면 강조한다: 텍스트 비교는 줄 안에서 바뀐 글자(`Some(0)`),
/// 정규식은 몇 번째 일치인지(0부터, 앱은 짝·홀로 색을 번갈아 쓴다)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<u32>,
}

/// `Records` 블록의 카드 하나
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// 카드 이름표 (예: `매치 1`)
    pub title: String,
    /// 오른쪽 보조 글 (예: `위치 13–29`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    /// 본문 (고정폭)
    pub text: String,
    /// 이름·값 줄 (예: `그룹 1` · `hello`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<RecordField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffTag {
    Equal,
    Delete,
    Insert,
}

/// 텍스트 비교의 한 줄 (통합 순서: 같은 줄, 지운 줄, 넣은 줄이 차례로 온다)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub tag: DiffTag,
    /// 원본 줄 번호 (1부터). `insert`는 없음
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<u32>,
    /// 비교 대상 줄 번호 (1부터). `delete`는 없음
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<u32>,
    /// 줄 내용 (줄바꿈 없음)
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffLayout {
    /// 좌우: 앱이 이어진 delete·insert 묶음을 한 줄씩 짝지어 나란히 그린다
    Split,
    /// 통합: 받은 순서대로 한 열에 그린다
    Unified,
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
        /// 저장할 때 기본 파일 이름 (예: `qr.png`). 비우면 [이미지 복사]·[저장]을 숨긴다 (보기만 하는 그림)
        name: String,
    },
    ImagePair {
        a: PathBuf,
        b: PathBuf,
        diff: PathBuf,
    },
    /// 강조 구간이 있는 글 (정규식 일치 표시). 조각을 이으면 원문
    Highlight {
        spans: Vec<Span>,
    },
    /// 표 + [복사](탭 구분). `name`이 있으면 [저장](CSV)
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// 카드 목록 + [복사] (정규식 일치 하나 = 카드 하나)
    Records {
        items: Vec<Record>,
        /// 있으면 [복사]가 카드 대신 이 글을 복사한다 (예: 대체텍스트의 이어 붙인 글)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        copy: Option<String>,
    },
    /// 텍스트 비교 결과
    Diff {
        layout: DiffLayout,
        lines: Vec<DiffLine>,
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
    fn new_blocks_wire_format() {
        let v = serde_json::to_value(Block::Diff {
            layout: DiffLayout::Split,
            lines: vec![DiffLine {
                tag: DiffTag::Delete,
                old: Some(1),
                new: None,
                spans: vec![Span {
                    text: "a".into(),
                    mark: Some(0),
                }],
            }],
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "type": "diff", "layout": "split", "lines": [
                { "tag": "delete", "old": 1, "spans": [{ "text": "a", "mark": 0 }] }
            ] })
        );
        let h = serde_json::to_value(Block::Highlight {
            spans: vec![Span {
                text: "x".into(),
                mark: None,
            }],
        })
        .unwrap();
        assert_eq!(
            h,
            serde_json::json!({ "type": "highlight", "spans": [{ "text": "x" }] })
        );
        assert_eq!(serde_json::to_value(ToolKind::Rename).unwrap(), "rename");
    }

    #[test]
    fn records_copy_is_optional_on_the_wire() {
        let b: Block =
            serde_json::from_value(serde_json::json!({ "type": "records", "items": [] })).unwrap();
        assert!(matches!(b, Block::Records { copy: None, .. }));
        assert_eq!(
            serde_json::to_value(&b).unwrap(),
            serde_json::json!({ "type": "records", "items": [] })
        );
    }

    #[test]
    fn kind_is_camel_case() {
        assert_eq!(serde_json::to_value(ToolKind::PerFile).unwrap(), "perFile");
    }
}
