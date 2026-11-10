import { invoke } from "@tauri-apps/api/core"

/** 앱 끝내기 (트레이 보관 설정과 상관없이). Tauri 밖에서는 아무것도 하지 않는다 */
export function quitApp() {
  invoke("quit_app").catch(() => {})
}
