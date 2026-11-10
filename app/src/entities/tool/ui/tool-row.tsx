import { IconStar, IconStarFilled } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import type { ToolEntry } from "../model/types"

type Props = {
  tool: ToolEntry
  selected: boolean
  favorite: boolean
  onSelect: () => void
  onToggleFavorite: () => void
  className?: string
}

export function ToolRow({
  tool,
  selected,
  favorite,
  onSelect,
  onToggleFavorite,
  className,
}: Props) {
  const Star = favorite ? IconStarFilled : IconStar
  return (
    <div
      className={cn(
        "group hover:bg-sidebar-accent my-px flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium",
        selected && "bg-selection hover:bg-selection",
        className,
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        className="flex min-w-0 flex-1 items-center gap-2 text-left"
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
