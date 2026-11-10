//! 기본 내장 도구 목록과 앱·CLI 공용 설정값.
//! 내장 ↔ 추가 설치 전환 시 `builtin()`과 Cargo 의존성, catalog/tools.json만 바꾼다.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use localtools_core::Tool;

pub fn builtin() -> Vec<Box<dyn Tool>> {
    vec![localtools_tool_dev_copy::tool()]
}

pub fn find(id: &str) -> Option<Box<dyn Tool>> {
    builtin().into_iter().find(|t| t.meta().id == id)
}

/// `catalog/tools.json`의 한 항목. 설치하지 않은 도구도 사이드바에 보이게 하는 목록.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// 앱에 기본 내장되는가
    pub builtin: bool,
}

fn catalog_all() -> Vec<CatalogEntry> {
    serde_json::from_str(include_str!("../../../catalog/tools.json"))
        .expect("catalog/tools.json 형식 오류")
}

/// 사이드바에 보일 전체 도구 목록. release 빌드에서는 개발용(`dev`) 도구를 뺀다.
pub fn catalog() -> Vec<CatalogEntry> {
    catalog_all()
        .into_iter()
        .filter(|e| cfg!(debug_assertions) || e.category != "dev")
        .collect()
}

/// `%LOCALAPPDATA%\LocalTools\logs` (없으면 임시 폴더 아래)
pub fn default_log_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LocalTools")
        .join("logs")
}

/// `YYYYMMDD-HHMMSS-mmm-N` (UTC). N은 프로세스 안 카운터라 같은 밀리초에도 겹치지 않는다.
pub fn new_job_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();
    let secs = ms / 1000;
    let (y, m, d) = ymd((secs / 86_400) as i64);
    let t = secs % 86_400;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}-{:03}-{n}",
        t / 3600,
        t / 60 % 60,
        t % 60,
        ms % 1000
    )
}

/// 1970-01-01부터의 일수 → (년, 월, 일). Howard Hinnant의 civil_from_days.
fn ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_ids_are_unique() {
        let ids: std::collections::HashSet<_> = (0..1000).map(|_| new_job_id()).collect();
        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn job_id_has_date_shape() {
        let id = new_job_id();
        let parts: Vec<_> = id.split('-').collect();
        assert_eq!(parts.len(), 4, "{id}");
        assert_eq!(parts[0].len(), 8);
        assert!(parts[0].starts_with("20"));
        assert_eq!(parts[1].len(), 6);
        assert_eq!(parts[2].len(), 3);
    }

    #[test]
    fn civil_date_is_correct() {
        // 2026-11-10 00:00:00 UTC
        assert_eq!(ymd(20767), (2026, 11, 10));
        assert_eq!(ymd(19417), (2023, 3, 1)); // 윤년이 아닌 해의 2월 다음 날
        assert_eq!(ymd(0), (1970, 1, 1));
    }

    #[test]
    fn find_returns_builtin_tool() {
        assert_eq!(find("dev.copy").unwrap().meta().id, "dev.copy");
        assert!(find("nope").is_none());
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalog_matches_builtin() {
        let cat = catalog_all();
        for t in builtin() {
            let m = t.meta();
            let e = cat
                .iter()
                .find(|e| e.id == m.id)
                .unwrap_or_else(|| panic!("{} 없음", m.id));
            assert!(e.builtin, "{}", m.id);
            assert_eq!(e.name, m.name);
        }
        for e in cat.iter().filter(|e| e.builtin) {
            assert!(find(&e.id).is_some(), "{} 구현 없음", e.id);
        }
    }

    #[test]
    fn catalog_ids_are_unique_and_well_formed() {
        let cat = catalog_all();
        let ids: std::collections::HashSet<_> = cat.iter().map(|e| &e.id).collect();
        assert_eq!(ids.len(), cat.len());
        for e in &cat {
            let (c, name) = e.id.split_once('.').unwrap_or_else(|| panic!("{}", e.id));
            assert_eq!(c, e.category, "{}", e.id);
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn catalog_has_twenty_image_tools() {
        assert_eq!(
            catalog_all()
                .iter()
                .filter(|e| e.category == "image")
                .count(),
            20
        );
    }
}
