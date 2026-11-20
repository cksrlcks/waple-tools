import { memo } from "react"

import { cn } from "@/shared/lib/utils"

import { scopeIds } from "../lib/scope-ids"

type Props = {
  cellKey: string
  svg: string
  title: string
  selected: boolean
  /** `selected`는 누르기 전 상태 (다시 누르면 해제) */
  onSelect: (key: string, selected: boolean) => void
  className?: string
}

/** 격자 한 칸: 정사각형 카드 가운데 아이콘, 카드 아래 이름. 다른 칸을 골라도 다시 그리지 않도록 memo */
export const IconGridCell = memo(function IconGridCell({
  cellKey,
  svg,
  title,
  selected,
  onSelect,
  className,
}: Props) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      aria-pressed={selected}
      onClick={() => onSelect(cellKey, selected)}
      className={cn(
        "group flex min-w-0 flex-col items-center gap-1.5 outline-none",
        className,
      )}
    >
      <span
        className={cn(
          "bg-card text-foreground group-hover:border-foreground/30 group-focus-visible:ring-primary/40 flex aspect-square w-full items-center justify-center rounded-lg border transition-colors group-focus-visible:ring-2",
          selected && "border-primary group-hover:border-primary",
        )}
        // svg는 DLL이 정리하고(crates/tools/web-icon-library/src/sanitize.rs) 이름도 검증한 것만 들어온다.
        // 한 화면에 여러 SVG가 있어 id가 부딪히지 않게 칸마다 접두어를 붙인다
        dangerouslySetInnerHTML={{ __html: scopeIds(svg, `c-${cellKey}`) }}
      />
      <span className="text-muted-foreground w-full truncate text-center text-xs">
        {title}
      </span>
    </button>
  )
})
