import { invoke } from "@tauri-apps/api/core"
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event"

/**
 * 옵션 패널 별도 창(`x-panel-position: "window"`, 창 라벨 `panel`)과 주고받는 것. Tauri 밖에서는 아무것도 하지 않는다.
 * 메인 → 창 `panel:state`(PanelState), 창 → 메인 `panel:ready` · `panel:set`({ key, value }) · `panel:close`,
 * 창이 닫히면 Rust가 메인에 `panel:closed`.
 */

/** 패널 창을 연다 (이미 있으면 앞으로) */
export function openPanelWindow(title: string) {
  invoke("open_panel_window", { title }).catch(() => {})
}

export function closePanelWindow() {
  invoke("close_panel_window").catch(() => {})
}

export function emitPanel(
  target: "main" | "panel",
  event: string,
  payload?: unknown,
) {
  emitTo(target, event, payload).catch(() => {})
}

/** 이벤트 여러 개를 듣는다. 모두 등록되면 `ready`. 돌려준 함수로 그만 듣는다 (등록 전에 불러도 된다) */
export function listenPanel(
  handlers: Record<string, (payload: unknown) => void>,
  ready?: () => void,
): () => void {
  let offs: UnlistenFn[] = []
  let stopped = false
  Promise.all(
    Object.entries(handlers).map(([event, fn]) =>
      listen(event, (e) => fn(e.payload)),
    ),
  )
    .then((fns) => {
      if (stopped) fns.forEach((f) => f())
      else {
        offs = fns
        ready?.()
      }
    })
    .catch(() => {})
  return () => {
    stopped = true
    offs.forEach((f) => f())
  }
}
