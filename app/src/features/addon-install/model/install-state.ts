import { useSyncExternalStore } from "react"

import type { DownloadProgress } from "@/entities/tool"

/** 설치 중인 도구와 진행률. 앱 전체에서 하나 (화면을 옮겨도 남는다). 없으면 설치 중 아님, null은 받기 전 */
const installs = new Map<string, DownloadProgress | null>()
const listeners = new Set<() => void>()
const subscribe = (l: () => void) => {
  listeners.add(l)
  return () => listeners.delete(l)
}

export function setInstallState(
  id: string,
  p: DownloadProgress | null | undefined,
) {
  if (p === undefined) installs.delete(id)
  else installs.set(id, p)
  listeners.forEach((l) => l())
}

export const isInstalling = (id: string) => installs.has(id)

export function useInstallState(id: string) {
  const p = useSyncExternalStore(subscribe, () => installs.get(id))
  return { installing: p !== undefined, progress: p ?? null }
}
