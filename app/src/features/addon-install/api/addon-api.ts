import { Channel, invoke } from "@tauri-apps/api/core"

import type { DownloadProgress } from "@/entities/tool"

/** 설치·업데이트·다시 설치 (받기 → 해시·서명 확인 → 로드) */
export function installTool(
  toolId: string,
  onProgress: (p: DownloadProgress) => void,
): Promise<void> {
  const channel = new Channel<DownloadProgress>()
  channel.onmessage = onProgress
  return invoke("install_tool", { toolId, onProgress: channel })
}

export function uninstallTool(toolId: string): Promise<void> {
  return invoke("uninstall_tool", { toolId })
}

/** 원격 도구 목록을 다시 받는다 (업데이트 표시용) */
export function refreshAddons(): Promise<void> {
  return invoke("refresh_addons")
}
