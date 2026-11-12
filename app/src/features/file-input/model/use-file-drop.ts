import { useEffect, useRef, useState, type RefObject } from "react"
import { getCurrentWebview } from "@tauri-apps/api/webview"

/**
 * 창에 파일·폴더를 끌어 놓으면 경로를 받는다. `enabled`가 false면(실행 중 등) 놓아도 무시한다.
 * `target`을 주면 그 요소 위에서 끌 때만 `over`, 그 위에 놓을 때만 받는다 (한 화면에 여러 칸).
 */
export function useFileDrop(
  onDrop: (paths: string[]) => void,
  enabled: boolean,
  target?: RefObject<HTMLElement | null>,
) {
  const [over, setOver] = useState(false)
  const latest = useRef({ onDrop, enabled, target })
  latest.current = { onDrop, enabled, target }

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false
    // Tauri 위치는 웹뷰 왼쪽 위 기준 물리 픽셀
    const inside = (p: { x: number; y: number }) => {
      const el = latest.current.target?.current
      if (!latest.current.target) return true
      if (!el) return false
      const r = el.getBoundingClientRect()
      const x = p.x / window.devicePixelRatio
      const y = p.y / window.devicePixelRatio
      return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
    }
    try {
      getCurrentWebview()
        .onDragDropEvent(({ payload }) => {
          if (payload.type === "enter" || payload.type === "over")
            setOver(inside(payload.position))
          else setOver(false)
          if (
            payload.type === "drop" &&
            latest.current.enabled &&
            inside(payload.position)
          )
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
