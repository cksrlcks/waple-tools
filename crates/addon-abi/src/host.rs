//! 앱 쪽: DLL을 읽어 `Tool`로 감싼다. 배치 엔진·UI는 기본 내장과 구분하지 않는다.

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::Library;
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

use crate::{
    ABI_VERSION, AbiVersionFn, FreeFn, Host, MetaFn, MetaJson, RunFn, RunRequest, RunResponse,
};

#[derive(Debug)]
pub enum LoadError {
    /// DLL을 열 수 없거나 함수가 없다
    Open(String),
    /// 다른 ABI 버전으로 빌드된 DLL (DLL의 버전)
    Abi(u32),
    /// 도구 정보 JSON이 잘못됐다
    Meta(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(e) => write!(f, "도구 파일을 열 수 없습니다: {e}"),
            Self::Abi(v) => write!(
                f,
                "앱과 맞지 않는 도구 파일입니다 (ABI {v}, 앱은 {ABI_VERSION})."
            ),
            Self::Meta(e) => write!(f, "도구 정보를 읽을 수 없습니다: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// 읽어 들인 추가 설치 도구. DLL은 이 값(과 복제본)이 모두 사라질 때까지 열려 있다.
pub struct AddonTool {
    info: MetaJson,
    run: RunFn,
    free: FreeFn,
    // 함수 포인터보다 나중에 해제되도록 마지막 필드
    _lib: Arc<Library>,
}

/// DLL을 연다.
///
/// # Safety
/// 서명을 확인한 파일만 넘긴다. DLL을 여는 것은 그 안의 코드를 실행하는 것이다.
pub unsafe fn load(path: &Path) -> Result<AddonTool, LoadError> {
    let open = |e: libloading::Error| LoadError::Open(e.to_string());
    unsafe {
        let lib = Library::new(path).map_err(open)?;
        let abi = *lib.get::<AbiVersionFn>(b"lt_abi_version\0").map_err(open)?;
        let v = abi();
        if v != ABI_VERSION {
            return Err(LoadError::Abi(v));
        }
        let meta = *lib.get::<MetaFn>(b"lt_meta\0").map_err(open)?;
        let run = *lib.get::<RunFn>(b"lt_run\0").map_err(open)?;
        let free = *lib.get::<FreeFn>(b"lt_free\0").map_err(open)?;
        let json = take(meta(), free);
        let info: MetaJson =
            serde_json::from_str(&json).map_err(|e| LoadError::Meta(e.to_string()))?;
        Ok(AddonTool {
            info,
            run,
            free,
            _lib: Arc::new(lib),
        })
    }
}

/// DLL이 만든 문자열을 복사하고 DLL에 돌려준다.
unsafe fn take(ptr: *mut c_char, free: FreeFn) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        free(ptr);
        s
    }
}

extern "C" fn host_progress(ctx: *const c_void, ratio: f32) {
    // 패닉이 C 경계를 넘으면 앱이 죽는다. progress 콜백은 배치 엔진의 것이라 패닉하지 않지만 그래도 막는다
    let _ = std::panic::catch_unwind(|| unsafe { &*(ctx as *const JobContext) }.progress(ratio));
}

extern "C" fn host_is_cancelled(ctx: *const c_void) -> bool {
    std::panic::catch_unwind(|| unsafe { &*(ctx as *const JobContext) }.is_cancelled())
        .unwrap_or(true)
}

impl Tool for AddonTool {
    fn meta(&self) -> ToolMeta {
        self.info.meta.clone()
    }

    fn kind(&self) -> ToolKind {
        self.info.kind
    }

    fn options_schema(&self) -> serde_json::Value {
        self.info.schema.clone()
    }

    fn run(
        &self,
        ctx: &JobContext,
        input: &[PathBuf],
        out_dir: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let req = serde_json::to_string(&RunRequest {
            input: input.to_vec(),
            out_dir: out_dir.to_path_buf(),
            opts: opts.clone(),
        })
        .map_err(|e| ToolError::Failed(e.to_string()))?;
        let req = CString::new(req).map_err(|e| ToolError::Failed(e.to_string()))?;
        let host = Host {
            ctx: ctx as *const JobContext as *const c_void,
            progress: host_progress,
            is_cancelled: host_is_cancelled,
        };
        let json = unsafe { take((self.run)(req.as_ptr(), &host), self.free) };
        serde_json::from_str::<RunResponse>(&json)
            .map_err(|_| ToolError::Failed("도구가 잘못된 결과를 돌려주었습니다.".into()))?
            .into()
    }
}
