//! 무료 Iconify 세트 목록. `scripts/update-icon-collections.mjs`가 만든 스냅샷을 바이너리에 담는다.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub total: u32,
    pub category: Option<String>,
    /// spdx 식별자
    pub license: String,
    pub palette: bool,
}

/// 이름순 전체 목록. 처음 부를 때 한 번만 읽는다.
pub fn all() -> &'static [Collection] {
    static LIST: OnceLock<Vec<Collection>> = OnceLock::new();
    LIST.get_or_init(|| {
        serde_json::from_str(include_str!("../data/collections.json"))
            .expect("data/collections.json이 올바르지 않습니다")
    })
}
