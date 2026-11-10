//! 오래된 작업 기록 정리

use std::path::Path;
use std::time::{Duration, SystemTime};

/// 수정 시각이 `older_than`보다 오래된 `*.jsonl`을 지우고 지운 수를 돌려준다. 폴더가 없으면 0.
pub fn prune_logs(log_dir: &Path, older_than: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return 0;
    };
    let Some(cutoff) = SystemTime::now().checked_sub(older_than) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .is_ok_and(|t| t < cutoff)
        })
        .filter(|p| std::fs::remove_file(p).is_ok())
        .count()
}
