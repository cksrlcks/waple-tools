//! 추가 도구 다운로드. 앱에서 네트워크를 쓰는 곳은 여기와 앱 업데이트(tauri-plugin-updater)뿐이다.
//! https만, 시작은 github.com, 리다이렉트는 *.githubusercontent.com까지. 진짜 보안 경계는 서명 확인이다.

use std::path::Path;

use reqwest::Url;
use reqwest::redirect::Policy;

pub const INDEX_URL: &str =
    "https://github.com/cksrlcks/waple-tools/releases/download/addons/index.json";

const OFFLINE: &str = "인터넷 연결이 필요합니다.";
const BAD_HOST: &str = "허용되지 않은 주소입니다.";
const UNAVAILABLE: &str = "추가 도구 목록을 받을 수 없습니다. 잠시 뒤 다시 시도해 주세요.";

pub fn allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|h| h == "github.com" || h.ends_with(".githubusercontent.com"))
}

/// `local_base`가 있으면(debug 빌드의 로컬 index) https가 아닌 주소를 그 폴더 기준 파일 경로로 읽는다.
pub async fn fetch(
    url: &str,
    local_base: Option<&Path>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    if let Some(base) = local_base.filter(|_| cfg!(debug_assertions))
        && !url.starts_with("https://")
    {
        let path = base.join(url);
        let bytes = std::fs::read(&path)
            .map_err(|_| format!("로컬 도구 파일을 읽을 수 없습니다: {}", path.display()))?;
        on_progress(bytes.len() as u64, Some(bytes.len() as u64));
        return Ok(bytes);
    }
    let url = Url::parse(url).map_err(|_| BAD_HOST.to_string())?;
    if !allowed(&url) {
        return Err(BAD_HOST.into());
    }
    // updater와 같은 방식: rustls 암호 공급자를 한 번 설치한다
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .user_agent("waple-tools")
        .redirect(Policy::custom(|a| {
            if !allowed(a.url()) {
                a.error(BAD_HOST)
            } else if a.previous().len() > 5 {
                a.stop()
            } else {
                a.follow()
            }
        }))
        .build()
        .map_err(|_| OFFLINE.to_string())?;
    // 연결이 안 됨 / 허용 밖으로 이동 / 서버가 오류로 답함(404 등)을 나눠 알린다
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| if e.is_redirect() { BAD_HOST } else { OFFLINE }.to_string())?;
    let mut resp = resp
        .error_for_status()
        .map_err(|_| UNAVAILABLE.to_string())?;
    let total = resp.content_length();
    let mut out = Vec::with_capacity(total.unwrap_or(0) as usize);
    while let Some(chunk) = resp.chunk().await.map_err(|_| OFFLINE.to_string())? {
        out.extend_from_slice(&chunk);
        on_progress(out.len() as u64, total);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_github_over_https() {
        let ok = |s: &str| allowed(&Url::parse(s).unwrap());
        assert!(ok(INDEX_URL));
        assert!(ok("https://release-assets.githubusercontent.com/x"));
        assert!(!ok("http://github.com/x"));
        assert!(!ok("https://github.com.evil.com/x"));
        assert!(!ok("https://evilgithubusercontent.com/x"));
    }
}
