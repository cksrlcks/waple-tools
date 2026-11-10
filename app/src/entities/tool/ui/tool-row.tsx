import { IconStar, IconStarFilled } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import { toolIcon } from "../config/tool-icons"
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
  const Icon = toolIcon(tool.id)
  const missing = tool.status === "notInstalled"
  return (
    <div
      className={cn(
        "group hover:bg-accent flex h-8 items-center gap-2 rounded-md pr-1.5 pl-2 text-sm transition-colors",
        selected && "bg-selection hover:bg-selection text-foreground",
        className,
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        disabled={disabled}
        aria-current={selected ? "page" : undefined}
        title={disabled ? "작업이 끝난 뒤 이동할 수 있습니다." : undefined}
        className="flex min-w-0 flex-1 items-center gap-2.5 text-left outline-none disabled:opacity-50"
      >
        <Icon
          className={cn(
            "size-4 shrink-0",
            selected ? "text-primary" : "text-muted-foreground",
            missing && "opacity-60",
          )}
          stroke={1.75}
        />
        <span
          className={cn(
            "truncate",
            selected && "font-medium",
            missing && "text-muted-foreground",
          )}
        >
          {tool.name}
        </span>
        {missing && (
          <span className="text-2xs text-muted-foreground bg-muted ml-auto shrink-0 rounded px-1.5 py-px group-hover:hidden">
            설치 필요
          </span>
        )}
      </button>
      <button
        type="button"
        aria-label={favorite ? "즐겨찾기에서 빼기" : "즐겨찾기에 추가"}
        onClick={onToggleFavorite}
        className={cn(
          "hover:text-foreground flex size-6 shrink-0 items-center justify-center rounded",
          favorite
            ? "text-primary"
            : "text-muted-foreground hidden group-hover:flex focus-visible:flex",
        )}
      >
        <Star className="size-3.5" stroke={1.75} />
      </button>
    </div>
  )
}
