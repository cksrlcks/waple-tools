import { invoke, isTauri } from "@tauri-apps/api/core"
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event"

/**
 * 옵션 패널 별도 창(`x-panel-position: "window"`, 창 라벨 `panel`)과 주고받는 것. Tauri 밖에서는 아무것도 하지 않는다.
 * 메인 → 창 `panel:state`(PanelState), 창 → 메인 `panel:ready` · `panel:set`({ key, value }) · `panel:close`,
 * 창이 없어지면(Alt+F4 등) Rust가 메인에 `panel:closed`.
 */

// 창 명령은 부른 순서대로 (보이기(async 명령)와 숨기기가 엇갈리지 않게)
let queue: Promise<unknown> = Promise.resolve()
function windowCommand(cmd: string, args?: Record<string, unknown>) {
  if (!isTauri()) return Promise.resolve()
  const p = queue.then(() => invoke<void>(cmd, args))
  queue = p.catch(() => {})
  return p
}

/** 패널 창을 보인다 (없으면 숨긴 채로 만들고, 창의 `panel:ready` 뒤에 다시 불러 보인다). 실패는 호출한 쪽이 알린다 */
export function showPanelWindow(title: string) {
  return windowCommand("show_panel_window", { title })
}

/** 선택 해제: 창은 남기고 숨긴다 */
export function hidePanelWindow() {
  windowCommand("hide_panel_window").catch(() => {})
}

/** 다른 도구로 갈 때: 창을 닫는다 */
export function closePanelWindow() {
  windowCommand("close_panel_window").catch(() => {})
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
