//! DLL 쪽: `export_tool!`이 부르는 함수. 패닉이 DLL 경계를 넘지 않게 모두 잡는다.

use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

use waple_tools_core::{JobContext, Tool};

use crate::{Host, MetaJson, RunRequest, RunResponse};

const PANIC: &str = "도구 실행 중 오류가 발생했습니다.";

fn to_c(json: String) -> *mut c_char {
    // serde_json은 문자열 안의 NUL을 \u0000으로 이스케이프하므로 실패하지 않는다
    CString::new(json).unwrap_or_default().into_raw()
}

pub fn meta(ctor: fn() -> Box<dyn Tool>) -> *mut c_char {
    let json = catch_unwind(|| {
        let t = ctor();
        serde_json::to_string(&MetaJson {
            meta: t.meta(),
            kind: t.kind(),
            schema: t.options_schema(),
        })
        .unwrap_or_default()
    })
    // 빈 문자열이면 앱이 "도구 정보를 읽을 수 없다"로 거부한다
    .unwrap_or_default();
    to_c(json)
}

/// 앱의 `ctx` 포인터를 다른 스레드(도구 내부 병렬 처리)에서도 쓸 수 있게 감싼다.
/// 앱은 `lt_run`이 끝날 때까지 `ctx`를 살려 두고, 콜백은 스레드 안전하다.
#[derive(Clone, Copy)]
struct SendHost {
    ctx: *const c_void,
    progress: extern "C" fn(*const c_void, f32),
    is_cancelled: extern "C" fn(*const c_void) -> bool,
}
unsafe impl Send for SendHost {}
unsafe impl Sync for SendHost {}

// 메서드로 부른다: 클로저가 필드(`*const c_void`)만 따로 잡으면 Send가 아니게 된다
impl SendHost {
    fn progress(self, r: f32) {
        (self.progress)(self.ctx, r)
    }
    fn is_cancelled(self) -> bool {
        (self.is_cancelled)(self.ctx)
    }
}

/// # Safety
/// `req`는 NUL로 끝나는 문자열, `host`는 호출 동안 유효해야 한다.
pub unsafe fn run(
    ctor: fn() -> Box<dyn Tool>,
    req: *const c_char,
    host: *const Host,
) -> *mut c_char {
    let (req, host) = unsafe { (CStr::from_ptr(req).to_bytes(), &*host) };
    let h = SendHost {
        ctx: host.ctx,
        progress: host.progress,
        is_cancelled: host.is_cancelled,
    };
    let resp = catch_unwind(AssertUnwindSafe(|| {
        let req: RunRequest = match serde_json::from_slice(req) {
            Ok(r) => r,
            Err(e) => return RunResponse::Failed(format!("실행 요청을 읽을 수 없습니다: {e}")),
        };
        let ctx = JobContext::with_cancel_fn(move |r| h.progress(r), move || h.is_cancelled());
        ctor().run(&ctx, &req.input, &req.out_dir, &req.opts).into()
    }))
    .unwrap_or_else(|_| RunResponse::Failed(PANIC.into()));
    to_c(serde_json::to_string(&resp).unwrap_or_default())
}

/// # Safety
/// `meta`·`run`이 돌려준 포인터만, 한 번씩.
pub unsafe fn free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
