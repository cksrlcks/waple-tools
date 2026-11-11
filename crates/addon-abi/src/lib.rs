//! 추가 설치 도구 DLL ↔ 앱 사이 C ABI (설계 문서 6.3).
//!
//! DLL 경계는 C 함수 + JSON 문자열로만 넘긴다 (Rust에는 안정된 ABI가 없다).
//!
//! | 내보내는 함수 | 역할 |
//! | --- | --- |
//! | `lt_abi_version() -> u32` | ABI 버전. 앱이 아는 버전과 다르면 로드 거부 |
//! | `lt_meta() -> *mut c_char` | [`MetaJson`] |
//! | `lt_run(req, host) -> *mut c_char` | [`RunRequest`] → [`RunResponse`] |
//! | `lt_free(ptr)` | DLL이 만든 문자열 해제 |
//!
//! 도구 크레이트는 `waple_tools_addon_abi::export_tool!(tool);` 한 줄을 두고,
//! `--features addon --crate-type cdylib`로 빌드하면 DLL이 된다. 그 밖의 빌드에서는 아무것도 내보내지 않는다.

use std::ffi::{c_char, c_void};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use waple_tools_core::{ToolError, ToolKind, ToolMeta, ToolOutput};

#[doc(hidden)]
pub mod guest;
#[cfg(feature = "host")]
mod host;
#[cfg(feature = "testkit")]
pub mod testkit;

#[cfg(feature = "host")]
pub use host::{AddonTool, LoadError, load};

/// 바꾸면 이미 설치된 DLL은 "업데이트 필요"가 된다. 릴리스 워크플로가 이 줄을 읽는다.
pub const ABI_VERSION: u32 = 1;

/// 앱이 `lt_run`에 넘기는 함수 묶음. `ctx`는 호출이 끝날 때까지만 유효하다.
#[repr(C)]
pub struct Host {
    pub ctx: *const c_void,
    pub progress: extern "C" fn(ctx: *const c_void, ratio: f32),
    pub is_cancelled: extern "C" fn(ctx: *const c_void) -> bool,
}

pub type AbiVersionFn = unsafe extern "C" fn() -> u32;
pub type MetaFn = unsafe extern "C" fn() -> *mut c_char;
pub type RunFn = unsafe extern "C" fn(req: *const c_char, host: *const Host) -> *mut c_char;
pub type FreeFn = unsafe extern "C" fn(ptr: *mut c_char);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaJson {
    pub meta: ToolMeta,
    pub kind: ToolKind,
    pub schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub input: Vec<PathBuf>,
    pub out_dir: PathBuf,
    pub opts: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunResponse {
    Ok(ToolOutput),
    Cancelled,
    InvalidOptions(String),
    /// 그 밖의 오류는 사용자에게 보일 문구 그대로
    Failed(String),
}

impl From<Result<ToolOutput, ToolError>> for RunResponse {
    fn from(r: Result<ToolOutput, ToolError>) -> Self {
        match r {
            Ok(out) => Self::Ok(out),
            Err(ToolError::Cancelled) => Self::Cancelled,
            Err(ToolError::InvalidOptions(m)) => Self::InvalidOptions(m),
            Err(e) => Self::Failed(e.to_string()),
        }
    }
}

impl From<RunResponse> for Result<ToolOutput, ToolError> {
    fn from(r: RunResponse) -> Self {
        match r {
            RunResponse::Ok(out) => Ok(out),
            RunResponse::Cancelled => Err(ToolError::Cancelled),
            RunResponse::InvalidOptions(m) => Err(ToolError::InvalidOptions(m)),
            RunResponse::Failed(m) => Err(ToolError::Failed(m)),
        }
    }
}

/// 도구 크레이트에서 `export_tool!(tool);` — `tool`은 `fn() -> Box<dyn Tool>`.
/// 그 크레이트의 `addon` 기능이 켜졌을 때만 C 함수를 내보낸다.
#[macro_export]
macro_rules! export_tool {
    ($ctor:path) => {
        #[cfg(feature = "addon")]
        const _: () = {
            use ::std::ffi::c_char;

            #[unsafe(no_mangle)]
            pub extern "C" fn lt_abi_version() -> u32 {
                $crate::ABI_VERSION
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn lt_meta() -> *mut c_char {
                $crate::guest::meta($ctor)
            }

            /// # Safety
            /// `req`는 NUL로 끝나는 UTF-8, `host`는 호출 동안 유효해야 한다.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn lt_run(
                req: *const c_char,
                host: *const $crate::Host,
            ) -> *mut c_char {
                unsafe { $crate::guest::run($ctor, req, host) }
            }

            /// # Safety
            /// `lt_meta`·`lt_run`이 돌려준 포인터만, 한 번씩.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn lt_free(ptr: *mut c_char) {
                unsafe { $crate::guest::free(ptr) }
            }
        };
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_round_trips_errors() {
        let r: RunResponse = Err::<ToolOutput, _>(ToolError::Io(std::io::Error::other("x"))).into();
        let json = serde_json::to_string(&r).unwrap();
        let back: Result<ToolOutput, ToolError> =
            serde_json::from_str::<RunResponse>(&json).unwrap().into();
        assert!(matches!(back, Err(ToolError::Failed(m)) if m == "파일 오류: x"));
    }

    #[test]
    fn response_round_trips_files() {
        let r = RunResponse::Ok(ToolOutput::Files(vec!["a/b.png".into()]));
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"ok":{"files":["a/b.png"]}}"#);
        let back: Result<ToolOutput, ToolError> =
            serde_json::from_str::<RunResponse>(&json).unwrap().into();
        assert!(matches!(back, Ok(ToolOutput::Files(f)) if f == [PathBuf::from("a/b.png")]));
    }
}
