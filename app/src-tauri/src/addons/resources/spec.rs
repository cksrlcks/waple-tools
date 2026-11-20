//! 도구 스키마의 `x-resources`: 도구가 받아 쓸 npm 패키지(리소스)의 규칙.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceSpec {
    /// npm 패키지 이름. `{id}` 하나를 리소스 id로 바꾼다.
    pub package: String,
    /// 패키지에서 저장할 파일 (패키지 루트 기준, 폴더 없이 이름만)
    pub files: Vec<String>,
}

pub fn parse(schema: &Value) -> Option<ResourceSpec> {
    let r = schema.get("x-resources")?;
    if r.get("source")?.as_str()? != "npm" {
        return None;
    }
    let package = r.get("package")?.as_str()?.to_string();
    if package.matches("{id}").count() != 1 {
        return None;
    }
    let files = r
        .get("files")?
        .as_array()?
        .iter()
        .map(|f| f.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    let ok = |f: &String| !f.is_empty() && !f.contains(['/', '\\']) && !f.contains("..");
    (!files.is_empty() && files.iter().all(ok)).then_some(ResourceSpec { package, files })
}
