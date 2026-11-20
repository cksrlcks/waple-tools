import { useEffect, useRef } from "react"

import {
  closePanelWindow,
  emitPanel,
  listenPanel,
  openPanelWindow,
} from "../api/panel-window"
import type { PanelState } from "../lib/panel-state"

/**
 * 옵션 패널을 별도 창으로 (`x-panel-position: "window"`). `state`가 있으면 창을 열고 바뀔 때마다 보내고,
 * null이 되거나 작업대가 사라지면(다른 도구) 닫는다. 창에서 바꾼 옵션은 `onSet`, 창을 닫으면 `onClose`.
 */
export function usePanelWindow(
  state: PanelState | null,
  onSet: (key: string, value: unknown) => void,
  onClose: () => void,
) {
  const open = state !== null
  const title = state?.title ?? ""
  useEffect(() => {
    if (!open) return
    openPanelWindow(title)
    return closePanelWindow
  }, [open, title])

  // 결과 블록 배열은 렌더마다 새로 만들어지므로 내용이 바뀔 때만 보낸다
  const json = state && JSON.stringify(state)
  const latest = useRef(state)
  latest.current = state
  const callbacks = useRef({ onSet, onClose })
  callbacks.current = { onSet, onClose }
  useEffect(() => {
    if (json) emitPanel("panel", "panel:state", latest.current)
  }, [json])

  useEffect(() => {
    if (!open) return
    return listenPanel({
      // 창이 막 떴다: 놓친 상태를 다시
      "panel:ready": () => emitPanel("panel", "panel:state", latest.current),
      "panel:set": (p) => {
        const { key, value } = p as { key: string; value: unknown }
        callbacks.current.onSet(key, value)
      },
      "panel:close": () => callbacks.current.onClose(),
      // X 대신 Alt+F4 등으로 닫힘 (Rust가 보낸다)
      "panel:closed": () => callbacks.current.onClose(),
    })
  }, [open])
}
