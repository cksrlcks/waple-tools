import { useState } from "react"

import { ToolRow, type ToolEntry } from "@/entities/tool"
import { VersionCheck } from "@/features/app-update"
import {
  filterTools,
  groupTools,
  ToolSearchInput,
} from "@/features/tool-search"
import { cn } from "@/shared/lib/utils"

type Props = {
  tools: ToolEntry[]
  favorites: string[]
  /** 설정 화면이면 null */
  selectedId: string | null
  onSelect: (id: string) => void
  onToggleFavorite: (id: string) => void
  /** 작업 중이면 다른 도구로 이동하지 못한다 */
  locked?: boolean
  className?: string
}

export function AppSidebar({
  tools,
  favorites,
  selectedId,
  onSelect,
  onToggleFavorite,
  locked = false,
  className,
}: Props) {
  const [query, setQuery] = useState("")
  const groups = groupTools(filterTools(tools, query), favorites)

  return (
    <aside
      className={cn(
        "bg-sidebar border-border flex w-60 shrink-0 flex-col border-r px-3 pt-4 pb-3",
        className,
      )}
    >
      <ToolSearchInput value={query} onChange={setQuery} className="mb-4" />
      <nav className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto">
        {groups.map((g) => (
          <section key={g.key} className="flex flex-col gap-px">
            <h2 className="text-2xs text-muted-foreground px-2 pb-1.5 font-medium">
              {g.label}
            </h2>
            {g.tools.map((t) => (
              <ToolRow
                key={t.id}
                tool={t}
                selected={t.id === selectedId}
                favorite={favorites.includes(t.id)}
                onSelect={() => onSelect(t.id)}
                disabled={locked && t.id !== selectedId}
                onToggleFavorite={() => onToggleFavorite(t.id)}
              />
            ))}
          </section>
        ))}
        {groups.length === 0 && (
          <p className="text-muted-foreground px-2">찾는 도구가 없습니다.</p>
        )}
      </nav>
      {/* 맨 아래: 버전 · 업데이트 확인 (u-drive와 같은 방식) */}
      <VersionCheck locked={locked} className="mt-auto px-2 pt-2" />
    </aside>
  )
}
