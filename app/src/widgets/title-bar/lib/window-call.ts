import { getCurrentWindow } from "@tauri-apps/api/window"

/** 현재 창 조작. Tauri 밖(브라우저 미리보기)에서는 아무것도 하지 않는다 */
export function windowCall(fn: "minimize" | "toggleMaximize" | "close") {
  try {
    getCurrentWindow()
      [fn]()
      .catch(() => {})
  } catch {
    // Tauri 밖
  }
}
