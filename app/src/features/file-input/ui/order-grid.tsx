import type { ReactNode } from "react"

import type { FileResult, PathStat } from "@/entities/tool"
import { formatBytes } from "@/shared/lib/format-bytes"
import { cn } from "@/shared/lib/utils"

import { useReorder } from "../model/use-reorder"
import { OrderCard } from "./order-card"

type Props = {
  items: PathStat[]
  /** 입력 경로 → 마지막 실행 결과 */
  results?: Map<string, FileResult>
  /** 없으면 순서 없는 카드 격자 (끌기·순번 없음) */
  onReorder?: (from: number, to: number) => void
  onRemove: (path: string) => void
  onClear: () => void
  /** 카드 그림 (다른 기능이 채운다) */
  renderThumb?: (item: PathStat) => ReactNode
  /** 격자 끝에 붙일 것 (추가 타일) */
  trailing?: ReactNode
  /** 카드 한 장을 작업대 가운데에, 세로로 긴 썸네일로 (한 파일만 받는 도구) */
  centered?: boolean
  disabled?: boolean
  className?: string
}

/** 작업대의 카드 격자: 넣은 파일을 카드로 늘어놓는다. `onReorder`가 있으면 끌어서(또는 Alt+←/→) 합칠 순서를 정한다 */
export function OrderGrid({
  items,
  results,
  onReorder,
  onRemove,
  onClear,
  renderThumb,
  trailing,
  centered,
  disabled,
  className,
}: Props) {
  const { drag, bind } = useReorder(disabled ? undefined : onReorder)
  const ordered = onReorder !== undefined
  const total = items.reduce((n, i) => n + (i.isDir ? 0 : i.size), 0)
  return (
    <section className={cn("flex min-h-0 flex-col", className)}>
      <div className="text-stage-muted flex h-11 shrink-0 items-center gap-3 px-4 text-xs">
        <span className="text-stage-foreground text-sm font-medium tabular-nums">
          {items.length}개 · {formatBytes(total)}
        </span>
        {ordered && (
          <span className="hidden sm:inline">
            이 순서대로 합칩니다. 카드를 끌어 순서를 바꾸세요.
          </span>
        )}
        <button
          type="button"
          onClick={onClear}
          disabled={disabled}
          className="hover:text-stage-foreground ml-auto disabled:opacity-50"
        >
          {centered ? "비우기" : "모두 비우기"}
        </button>
      </div>
      <ul
        className={cn(
          "min-h-0 flex-1 overflow-y-auto px-4 pt-1 pb-4",
          centered
            ? "flex items-center justify-center *:w-40"
            : "grid auto-rows-min grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-4",
        )}
      >
        {items.map((item, index) => (
          <OrderCard
            key={item.path}
            item={item}
            order={ordered ? index + 1 : undefined}
            result={results?.get(item.path)}
            thumb={renderThumb?.(item)}
            dragging={drag?.from === index}
            target={
              drag && drag.over === index && drag.from !== index
                ? drag.from < index
                  ? "after"
                  : "before"
                : null
            }
            onRemove={() => onRemove(item.path)}
            disabled={disabled}
            reorder={ordered ? bind(index) : undefined}
            tall={centered}
          />
        ))}
        {trailing && <li>{trailing}</li>}
      </ul>
    </section>
  )
}
