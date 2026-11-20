import { useEffect, useRef } from "react"
import { toast } from "sonner"

import { toUserMessage } from "@/shared/lib/to-user-message"

import {
  closePanelWindow,
  emitPanel,
  hidePanelWindow,
  listenPanel,
  showPanelWindow,
} from "../api/panel-window"
import type { PanelState } from "../lib/panel-state"

/**
 * 옵션 패널을 별도 창으로 (`x-panel-position: "window"`면 `enabled`). 창은 처음 필요할 때 한 번 만들고 다시 쓴다:
 * `state`가 있으면 보이고 바뀔 때마다 보내고, null이면 숨긴다. 작업대가 사라지면(다른 도구) 닫는다.
 * 창에서 바꾼 옵션은 `onSet`, 창의 X·Alt+F4나 창을 못 열면 `onClose`.
 */
export function usePanelWindow(
  enabled: boolean,
  state: PanelState | null,
  onSet: (key: string, value: unknown) => void,
  onClose: () => void,
) {
  const latest = useRef(state)
  latest.current = state
  const callbacks = useRef({ onSet, onClose })
  callbacks.current = { onSet, onClose }

  const shown = state !== null
  const title = state?.title ?? ""
  useEffect(() => {
    if (!enabled) return
    if (!shown) return hidePanelWindow()
    showPanelWindow(title).catch((err: unknown) => {
      toast.error(toUserMessage(err))
      callbacks.current.onClose()
    })
  }, [enabled, shown, title])

  // 결과 블록 배열은 렌더마다 새로 만들어지므로 내용이 바뀔 때만 보낸다
  const json = state && JSON.stringify(state)
  useEffect(() => {
    if (enabled && json) emitPanel("panel", "panel:state", latest.current)
  }, [enabled, json])

  useEffect(() => {
    if (!enabled) return
    const off = listenPanel({
      // 창이 막 떴다(숨긴 채로 만들어짐): 상태를 보내고 보인다. 그새 선택이 풀렸으면 숨긴 채로
      "panel:ready": () => {
        const s = latest.current
        if (!s) return hidePanelWindow()
        emitPanel("panel", "panel:state", s)
        showPanelWindow(s.title).catch(() => {})
      },
      "panel:set": (p) => {
        const { key, value } = p as { key: string; value: unknown }
        callbacks.current.onSet(key, value)
      },
      // X: 창은 스스로 숨는다
      "panel:close": () => callbacks.current.onClose(),
      // Alt+F4 등으로 창이 없어짐 (Rust가 보낸다). 다음에 고르면 새로 만든다
      "panel:closed": () => callbacks.current.onClose(),
    })
    return () => {
      off()
      closePanelWindow()
    }
  }, [enabled])
}
