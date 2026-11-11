//! 테스트용 더미 도구: 입력 파일을 out_dir에 그대로 복사한다.

use std::path::{Path, PathBuf};

use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CopyOptions {
    /// 일부러 패닉 (추가 설치 DLL이 패닉해도 앱이 죽지 않는지 확인용, 화면에는 안 보임)
    #[serde(default)]
    #[schemars(skip)]
    panic: bool,
}

struct DummyCopy;

pub fn tool() -> Box<dyn Tool> {
    Box::new(DummyCopy)
}

waple_tools_addon_abi::export_tool!(tool);

impl Tool for DummyCopy {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "dev.copy".into(),
            name: "파일 복사(테스트)".into(),
            category: "dev".into(),
            description: "입력 파일을 그대로 복사합니다.".into(),
            extensions: vec![],
        }
    }

    fn kind(&self) -> ToolKind {
        ToolKind::PerFile
    }

    fn options_schema(&self) -> serde_json::Value {
        schemars::schema_for!(CopyOptions).to_value()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let opts: CopyOptions = serde_json::from_value(opts.clone())
            .map_err(|e| ToolError::InvalidOptions(e.to_string()))?;
        if opts.panic {
            panic!("일부러 낸 패닉");
        }
        let mut outputs = Vec::with_capacity(input.len());
        for (i, src) in input.iter().enumerate() {
            ctx.check_cancel()?;
            let name = src
                .file_name()
                .ok_or_else(|| ToolError::Failed("파일 이름이 없습니다.".into()))?;
            std::fs::copy(src, out_dir.join(name))?;
            outputs.push(PathBuf::from(name));
            ctx.progress((i + 1) as f32 / input.len() as f32);
        }
        Ok(ToolOutput::Files(outputs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use waple_tools_core::{JobContext, ToolError, ToolOutput};

    #[test]
    fn copies_into_out_dir_and_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("내 사진 a b.png");
        std::fs::write(&src, b"abc").unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let ctx = JobContext::new(|_| {}, Default::default());
        let r = tool()
            .run(
                &ctx,
                std::slice::from_ref(&src),
                &out,
                &serde_json::json!({}),
            )
            .unwrap();
        assert!(matches!(r, ToolOutput::Files(ref f) if f == &[PathBuf::from("내 사진 a b.png")]));
        assert_eq!(std::fs::read(out.join("내 사진 a b.png")).unwrap(), b"abc");
        assert_eq!(std::fs::read(&src).unwrap(), b"abc");
    }

    #[test]
    fn cancelled_before_start_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        std::fs::write(&src, b"abc").unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let ctx = JobContext::new(|_| {}, Arc::new(AtomicBool::new(true)));
        let r = tool().run(&ctx, &[src], &out, &serde_json::json!({}));
        assert!(matches!(r, Err(ToolError::Cancelled)));
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
    }

    #[test]
    fn meta_matches_spec() {
        let t = tool();
        assert_eq!(t.meta().id, "dev.copy");
        assert_eq!(t.meta().name, "파일 복사(테스트)");
        assert_eq!(t.kind(), waple_tools_core::ToolKind::PerFile);
    }
}
