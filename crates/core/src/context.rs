use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ToolError;

type ProgressFn = dyn Fn(f32) + Send + Sync;
type CancelFn = dyn Fn() -> bool + Send + Sync;

/// 도구 실행 중 진행률 보고와 취소 확인 통로.
pub struct JobContext {
    on_progress: Box<ProgressFn>,
    cancelled: Box<CancelFn>,
}

impl JobContext {
    pub fn new(on_progress: impl Fn(f32) + Send + Sync + 'static, cancel: Arc<AtomicBool>) -> Self {
        Self::with_cancel_fn(on_progress, move || cancel.load(Ordering::Relaxed))
    }

    /// 취소 여부를 함수로 묻는다 (추가 설치 도구: DLL이 앱에 물어본다)
    pub fn with_cancel_fn(
        on_progress: impl Fn(f32) + Send + Sync + 'static,
        cancelled: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            on_progress: Box::new(on_progress),
            cancelled: Box::new(cancelled),
        }
    }

    /// 0.0~1.0 비율. 범위를 벗어나면 잘라낸다.
    pub fn progress(&self, ratio: f32) {
        (self.on_progress)(ratio.clamp(0.0, 1.0));
    }

    pub fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }

    pub fn check_cancel(&self) -> Result<(), ToolError> {
        if self.is_cancelled() {
            Err(ToolError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolError;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn progress_is_clamped() {
        let seen = Arc::new(Mutex::new(vec![]));
        let s = seen.clone();
        let ctx = JobContext::new(
            move |r| s.lock().unwrap().push(r),
            Arc::new(AtomicBool::new(false)),
        );
        ctx.progress(-1.0);
        ctx.progress(0.5);
        ctx.progress(2.0);
        assert_eq!(*seen.lock().unwrap(), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn check_cancel_reports_cancelled() {
        let flag = Arc::new(AtomicBool::new(false));
        let ctx = JobContext::new(|_| {}, flag.clone());
        assert!(ctx.check_cancel().is_ok());
        flag.store(true, Ordering::Relaxed);
        assert!(ctx.is_cancelled());
        assert!(matches!(ctx.check_cancel(), Err(ToolError::Cancelled)));
    }
}
