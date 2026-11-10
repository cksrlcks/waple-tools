import { IconStar, IconStarFilled } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import type { ToolEntry } from "../model/types"

type Props = {
  tool: ToolEntry
  selected: boolean
  favorite: boolean
  onSelect: () => void
  onToggleFavorite: () => void
  /** 작업 중 다른 도구로 이동을 막을 때 */
  disabled?: boolean
  className?: string
}

export function ToolRow({
  tool,
  selected,
  favorite,
  onSelect,
  onToggleFavorite,
  disabled = false,
  className,
}: Props) {
  const Star = favorite ? IconStarFilled : IconStar
  return (
    <div
      className={cn(
        "group hover:bg-accent my-px flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium",
        selected && "bg-selection hover:bg-selection",
        className,
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        disabled={disabled}
        title={disabled ? "작업이 끝난 뒤 이동할 수 있습니다." : undefined}
        className="flex min-w-0 flex-1 items-center gap-2 text-left disabled:opacity-50"
      >
        <span className="truncate">{tool.name}</span>
        {tool.status === "notInstalled" && (
          <span className="text-2xs text-muted-foreground shrink-0 font-normal">
            설치 안 됨
          </span>
        )}
      </button>
      <button
        type="button"
        aria-label={favorite ? "즐겨찾기에서 빼기" : "즐겨찾기에 추가"}
        onClick={onToggleFavorite}
        className={cn(
          "text-muted-foreground hover:text-foreground shrink-0",
          favorite
            ? "text-primary"
            : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
        )}
      >
        <Star className="size-4" stroke={1.75} />
      </button>
    </div>
  )
}
