#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("취소됨")]
    Cancelled,
    #[error("옵션이 올바르지 않습니다: {0}")]
    InvalidOptions(String),
    #[error("파일 오류: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Failed(String),
}
