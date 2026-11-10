//! 화면용 작업(썸네일·미리보기)의 동시 실행 제한.
//! 사진 수백 장을 한꺼번에 넣어도 원본 디코딩이 CPU 코어 수만큼만 돌아 메모리가 폭주하지 않게 한다.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, LazyLock, Mutex};

pub struct Limiter {
    free: Mutex<usize>,
    cv: Condvar,
}

pub struct Slot<'a>(&'a Limiter);

impl Limiter {
    pub fn new(slots: usize) -> Self {
        Self {
            free: Mutex::new(slots.max(1)),
            cv: Condvar::new(),
        }
    }

    /// 자리가 날 때까지 기다렸다가 차지한다. 돌려받은 `Slot`이 드롭되면 자리를 비운다.
    pub fn acquire(&self) -> Slot<'_> {
        let mut free = self.free.lock().unwrap_or_else(|e| e.into_inner());
        while *free == 0 {
            free = self.cv.wait(free).unwrap_or_else(|e| e.into_inner());
        }
        *free -= 1;
        Slot(self)
    }
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        self.0.cv.notify_one();
    }
}

/// 썸네일·미리보기가 함께 쓰는 자리 (CPU 코어 수 - 1)
pub static VIEW: LazyLock<Limiter> =
    LazyLock::new(|| Limiter::new(waple_tools_core::batch::effective_threads(0)));

/// 미리보기는 한 번에 하나만, 그리고 가장 최근 요청만 돌린다
pub static PREVIEW_LOCK: Mutex<()> = Mutex::new(());
pub static PREVIEW_GEN: AtomicU64 = AtomicU64::new(0);

/// 새 미리보기 요청 번호
pub fn next_preview() -> u64 {
    PREVIEW_GEN.fetch_add(1, Ordering::SeqCst) + 1
}

/// 기다리는 동안 더 새 요청이 들어왔으면 이 요청은 건너뛴다
pub fn is_latest_preview(id: u64) -> bool {
    PREVIEW_GEN.load(Ordering::SeqCst) == id
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn limiter_caps_concurrency() {
        let limiter = Arc::new(Limiter::new(2));
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (l, r, p) = (limiter.clone(), running.clone(), peak.clone());
                std::thread::spawn(move || {
                    let _slot = l.acquire();
                    let now = r.fetch_add(1, Ordering::SeqCst) + 1;
                    p.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(30));
                    r.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn only_latest_preview_runs() {
        let a = next_preview();
        let b = next_preview();
        assert!(!is_latest_preview(a));
        assert!(is_latest_preview(b));
    }
}
