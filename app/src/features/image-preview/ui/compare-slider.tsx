import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react"

import type { Shown } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { clampPercent } from "../lib/clamp-percent"
import { CompareLabel } from "./compare-label"

type Props = {
  before: Shown
  after: Shown
  className?: string
}

/** 원본과 결과를 겹쳐 놓고 가운데 막대를 끌어 비교한다 (막대 왼쪽 = 원본, 오른쪽 = 결과) */
export function CompareSlider({ before, after, className }: Props) {
  const box = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState(50)
  const [dragging, setDragging] = useState(false)

  const moveTo = (clientX: number) => {
    const r = box.current?.getBoundingClientRect()
    if (r) setPos(clampPercent(clientX - r.left, r.width))
  }
  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId)
    setDragging(true)
    moveTo(e.clientX)
  }
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowLeft") setPos((p) => Math.max(0, p - 5))
    if (e.key === "ArrowRight") setPos((p) => Math.min(100, p + 5))
  }

  return (
    <div
      ref={box}
      className={cn(
        "bg-muted relative aspect-[4/3] w-full cursor-ew-resize overflow-hidden rounded-md select-none",
        className,
      )}
      onPointerDown={onPointerDown}
      onPointerMove={(e) => dragging && moveTo(e.clientX)}
      onPointerUp={() => setDragging(false)}
    >
      <img
        src={before.url}
        alt="원본"
        draggable={false}
        className="absolute inset-0 size-full object-contain"
      />
      <img
        src={after.url}
        alt="결과"
        draggable={false}
        className="absolute inset-0 size-full object-contain"
        style={{ clipPath: `inset(0 0 0 ${pos}%)` }}
      />
      <div
        role="slider"
        tabIndex={0}
        aria-label="원본과 결과 비교 위치"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(pos)}
        onKeyDown={onKeyDown}
        className="bg-background focus-visible:ring-primary/40 absolute inset-y-0 w-0.5 -translate-x-1/2 outline-none focus-visible:ring-2"
        style={{ left: `${pos}%` }}
      >
        <span className="bg-background shadow-item absolute top-1/2 left-1/2 size-4 -translate-1/2 rounded-full" />
      </div>
      <CompareLabel className="top-2 left-2">원본</CompareLabel>
      <CompareLabel className="top-2 right-2">결과</CompareLabel>
    </div>
  )
}
