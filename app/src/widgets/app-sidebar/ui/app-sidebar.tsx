import { useState } from "react"

import { ToolRow, type ToolEntry } from "@/entities/tool"
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
        "bg-background flex w-56 shrink-0 flex-col px-2.5 pt-3 pb-2",
        className,
      )}
    >
      <ToolSearchInput value={query} onChange={setQuery} className="mb-3" />
      <nav className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
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
    </aside>
  )
}
