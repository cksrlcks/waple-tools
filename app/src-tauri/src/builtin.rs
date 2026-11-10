use localtools_core::Tool;

/// 앱에 기본 내장되는 도구 목록. 내장 ↔ 추가 설치 전환 시 여기와 catalog/tools.json만 바꾼다.
pub fn builtin() -> Vec<Box<dyn Tool>> {
    vec![localtools_tool_dummy_copy::tool()]
}
