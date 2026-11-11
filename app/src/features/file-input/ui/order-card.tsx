import type { HTMLAttributes, ReactNode } from "react"
import {
  IconAlertCircleFilled,
  IconCircleCheckFilled,
  IconX,
} from "@tabler/icons-react"

import type { FileResult, PathStat } from "@/entities/tool"
import { baseName } from "@/shared/lib/base-name"
import { cn } from "@/shared/lib/utils"

import { statusOf } from "../lib/status-of"

type Props = {
  item: PathStat
  /** 1부터 시작하는 순서 */
  order: number
  result?: FileResult
  thumb?: ReactNode
  /** 끌고 있는 카드 */
  dragging?: boolean
  /** 놓을 자리 표시: 카드의 왼쪽/오른쪽 */
  target?: "before" | "after" | null
  onRemove: () => void
  disabled?: boolean
  /** 끌기·키보드 순서 바꾸기 핸들러 (`useReorder().bind`) */
  reorder?: HTMLAttributes<HTMLLIElement>
  className?: string
}

/** 순서 격자의 카드 한 장: 썸네일 · 순서 번호 · 이름 · 상태 */
export function OrderCard({
  item,
  order,
  result,
  thumb,
  dragging,
  target,
  onRemove,
  disabled,
  reorder,
  className,
}: Props) {
  const s = statusOf(item, result)
  return (
    <li
      {...reorder}
      className={cn(
        "group relative min-w-0",
        !disabled && "cursor-grab touch-none select-none",
        dragging && "opacity-50",
        className,
      )}
    >
      {target && (
        <span
          aria-hidden
          className={cn(
            "bg-primary absolute top-0 h-40 w-0.5 rounded-full",
            target === "before" ? "-left-2" : "-right-2",
          )}
        />
      )}
      <div
        tabIndex={0}
        title={`${item.path}\n끌거나 Alt+←/→ 로 순서 바꾸기`}
        className="focus-visible:ring-primary rounded-lg outline-none focus-visible:ring-2"
      >
        <span className="group-hover:ring-stage-foreground/25 relative block h-40 overflow-hidden rounded-lg ring-2 ring-transparent transition-shadow">
          {thumb}
          <span className="bg-stage/80 text-stage-foreground absolute top-1.5 left-1.5 flex h-5 min-w-5 items-center justify-center rounded-full px-1.5 text-xs font-medium tabular-nums backdrop-blur-sm">
            {order}
          </span>
        </span>
        <span className="mt-1.5 block truncate text-xs">
          {baseName(item.path)}
        </span>
        <span
          className={cn(
            "text-2xs flex items-center gap-1 truncate tabular-nums",
            s.tone === "error"
              ? "text-destructive"
              : s.tone === "done"
                ? "text-success"
                : "text-stage-muted",
          )}
          title={s.text}
        >
          {s.tone === "done" && (
            <IconCircleCheckFilled className="size-3 shrink-0" />
          )}
          {s.tone === "error" && (
            <IconAlertCircleFilled className="size-3 shrink-0" />
          )}
          <span className="truncate">{s.text}</span>
        </span>
      </div>
      <button
        type="button"
        aria-label="목록에서 빼기"
        onClick={onRemove}
        onPointerDown={(e) => e.stopPropagation()}
        disabled={disabled}
        className="bg-stage/80 text-stage-foreground hover:bg-stage absolute top-1.5 right-1.5 hidden size-5 items-center justify-center rounded-full backdrop-blur-sm group-hover:flex focus-visible:flex disabled:hidden"
      >
        <IconX className="size-3" stroke={2} />
      </button>
    </li>
  )
}
