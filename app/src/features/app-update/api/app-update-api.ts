import { relaunch } from "@tauri-apps/plugin-process"
import { check, type Update } from "@tauri-apps/plugin-updater"

/** GitHub Releases의 latest.json으로 새 버전 확인. 없으면 null */
export function checkUpdate(): Promise<Update | null> {
  return check()
}

/** 설치가 끝나면 앱을 다시 시작한다 */
export function restartApp(): Promise<void> {
  return relaunch()
}

export type { Update }
