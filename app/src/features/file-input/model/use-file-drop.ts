import { useEffect, useRef, useState } from "react"
import { getCurrentWebview } from "@tauri-apps/api/webview"

/** 창에 파일·폴더를 끌어 놓으면 경로를 받는다. `enabled`가 false면(실행 중 등) 놓아도 무시한다. */
export function useFileDrop(
  onDrop: (paths: string[]) => void,
  enabled: boolean,
) {
  const [over, setOver] = useState(false)
  const latest = useRef({ onDrop, enabled })
  latest.current = { onDrop, enabled }

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false
    try {
      getCurrentWebview()
        .onDragDropEvent(({ payload }) => {
          if (payload.type === "enter" || payload.type === "over") setOver(true)
          else setOver(false)
          if (payload.type === "drop" && latest.current.enabled)
            latest.current.onDrop(payload.paths)
        })
        .then((fn) => (cancelled ? fn() : (unlisten = fn)))
        .catch(() => {})
    } catch {
      // Tauri 밖(브라우저 미리보기)에서는 드롭을 쓰지 않는다
    }
    return () => {
      cancelled = true
      try {
        unlisten?.()
      } catch {
        // 앱이 닫히는 중이거나 Tauri 밖이면 해제할 것이 없다
      }
    }
  }, [])

  return { over }
}
