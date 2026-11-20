import { memo } from "react"

import { toSvg, type IconItem, type IconSet } from "@/entities/icon-set"
import { cn } from "@/shared/lib/utils"

import { scopeIds } from "../lib/scope-ids"

type Props = {
  icon: IconItem
  set: IconSet
  selected: boolean
  onSelect: (icon: IconItem) => void
  className?: string
}

/** 격자 한 칸. 크기·두께·색을 바꿔도 다시 그리지 않도록 memo */
export const IconCell = memo(function IconCell({
  icon,
  set,
  selected,
  onSelect,
  className,
}: Props) {
  const svg = scopeIds(
    toSvg(icon, set, { size: 24, stroke: null, color: "currentColor" }),
    `i-${icon.name}`,
  )
  return (
    <button
      type="button"
      title={icon.name}
      aria-label={icon.name}
      aria-pressed={selected}
      onClick={() => onSelect(icon)}
      className={cn(
        "hover:bg-stage-foreground/8 focus-visible:outline-primary flex aspect-square items-center justify-center rounded-md transition-colors outline-none focus-visible:outline-2",
        selected &&
          "bg-stage-foreground/15 outline-primary outline-2 -outline-offset-2",
        className,
      )}
      // body는 Rust에서 정리된 것만 들어온다 (app/src-tauri/src/addons/icons/sanitize.rs)
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  )
})
