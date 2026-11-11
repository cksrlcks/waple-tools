import { IconStar, IconStarFilled } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import { toolIcon } from "../config/tool-icons"
import type { ToolEntry } from "../model/types"

const BADGES: Partial<Record<ToolEntry["status"], string>> = {
  planned: "준비 중",
  notInstalled: "설치 필요",
  updateRequired: "다시 설치",
  updateAvailable: "업데이트",
}

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
  // 아직 쓸 수 없는 도구 (설치 전, 다시 설치해야 함)
  const missing =
    tool.status === "planned" ||
    tool.status === "notInstalled" ||
    tool.status === "updateRequired"
  const badge = BADGES[tool.status]
  return (
    <div
      className={cn(
        "group hover:bg-accent relative flex h-9 items-center gap-2 rounded-md pr-1.5 pl-2.5 text-sm leading-none transition-colors",
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
        // ::after 를 행 전체로 늘려 hover 영역과 클릭 영역을 맞춘다
        className="flex min-w-0 flex-1 items-center gap-3 text-left outline-none after:absolute after:inset-0 disabled:opacity-50"
      >
        <Icon
          className={cn(
            // 맑은 고딕은 글자가 아래로 치우쳐 그려져, 아이콘을 1px 내려 눈높이를 맞춘다
            "size-4 shrink-0 translate-y-px",
            selected ? "text-primary" : "text-muted-foreground",
            missing && "opacity-60",
          )}
          stroke={1.75}
        />
        <span
          className={cn(
            "truncate",
            selected && "font-bold",
            // 고른 행은 설치 전이어도 검정 (회색은 고르지 않은 행만)
            missing && !selected && "text-muted-foreground",
          )}
        >
          {tool.name}
        </span>
        {badge && (
          <span
            className={cn(
              "text-2xs ml-auto shrink-0 rounded px-1.5 py-px group-hover:hidden",
              tool.status === "updateAvailable"
                ? "text-primary bg-primary/10"
                : "text-muted-foreground bg-muted",
            )}
          >
            {badge}
          </span>
        )}
      </button>
      <button
        type="button"
        aria-label={favorite ? "즐겨찾기에서 빼기" : "즐겨찾기에 추가"}
        onClick={onToggleFavorite}
        className={cn(
          "hover:text-foreground relative flex size-6 shrink-0 items-center justify-center rounded",
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
