import {
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
} from "react"

/** 끌기를 시작했다고 보는 거리(px). 그보다 짧으면 누르기(선택)로 본다 */
const DRAG_START = 6

export type ReorderState = { from: number; over: number } | null

/**
 * 가로 카드 목록을 끌어서 순서 바꾸기. 포인터 이벤트로 직접 구현한다:
 * Tauri 파일 드롭이 켜져 있으면 Windows에서 HTML5 드래그가 동작하지 않는다.
 * 키보드는 카드에 초점을 두고 Alt+←/→.
 * `onReorder`가 없으면 아무것도 하지 않는다.
 */
export function useReorder(onReorder?: (from: number, to: number) => void) {
  const [drag, setDrag] = useState<ReorderState>(null)
  const start = useRef<{ x: number; y: number; index: number } | null>(null)
  // 끌기가 있었는지: 손을 뗀 뒤 오는 클릭을 막는 데 쓴다 (상태는 이미 지워져 있다)
  const moved = useRef(false)

  const reset = () => {
    start.current = null
    setDrag(null)
  }

  const bind = (index: number) => {
    if (!onReorder) return {}
    return {
      "data-reorder-index": index,
      onPointerDown: (e: PointerEvent<HTMLElement>) => {
        if (e.button !== 0) return
        start.current = { x: e.clientX, y: e.clientY, index }
        moved.current = false
      },
      onPointerMove: (e: PointerEvent<HTMLElement>) => {
        const s = start.current
        if (!s) return
        if (!drag) {
          if (Math.hypot(e.clientX - s.x, e.clientY - s.y) < DRAG_START) return
          e.currentTarget.setPointerCapture(e.pointerId)
          moved.current = true
          setDrag({ from: s.index, over: s.index })
          return
        }
        const el = document
          .elementFromPoint(e.clientX, e.clientY)
          ?.closest("[data-reorder-index]")
        if (!el) return
        const over = Number(el.getAttribute("data-reorder-index"))
        if (over !== drag.over) setDrag({ ...drag, over })
      },
      onPointerUp: () => {
        if (drag && drag.from !== drag.over) onReorder(drag.from, drag.over)
        reset()
      },
      onPointerCancel: reset,
      // 썸네일 <img>를 누른 채 움직이면 브라우저가 이미지 끌기를 시작하고 pointercancel을 보낸다: 막는다
      onDragStart: (e: DragEvent<HTMLElement>) => e.preventDefault(),
      // 끌고 난 뒤의 클릭은 선택으로 치지 않는다
      onClickCapture: (e: MouseEvent<HTMLElement>) => {
        if (!moved.current) return
        moved.current = false
        e.preventDefault()
        e.stopPropagation()
      },
      onKeyDown: (e: KeyboardEvent<HTMLElement>) => {
        if (!e.altKey || (e.key !== "ArrowLeft" && e.key !== "ArrowRight"))
          return
        e.preventDefault()
        onReorder(index, index + (e.key === "ArrowLeft" ? -1 : 1))
      },
    }
  }

  return { drag, bind }
}
