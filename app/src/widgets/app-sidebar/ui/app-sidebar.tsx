import { useState } from "react"
import { IconSettings } from "@tabler/icons-react"

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
  settingsOpen: boolean
  onSelect: (id: string) => void
  onOpenSettings: () => void
  onToggleFavorite: (id: string) => void
  className?: string
}

export function AppSidebar({
  tools,
  favorites,
  selectedId,
  settingsOpen,
  onSelect,
  onOpenSettings,
  onToggleFavorite,
  className,
}: Props) {
  const [query, setQuery] = useState("")
  const groups = groupTools(filterTools(tools, query), favorites)

  return (
    <aside
      className={cn(
        "border-border bg-sidebar flex w-60 shrink-0 flex-col border-r p-2",
        className,
      )}
    >
      <ToolSearchInput value={query} onChange={setQuery} className="mb-2" />
      <nav className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        {groups.map((g) => (
          <section key={g.key}>
            <h2 className="text-2xs text-muted-foreground px-2 pb-1 font-medium">
              {g.label}
            </h2>
            {g.tools.map((t) => (
              <ToolRow
                key={t.id}
                tool={t}
                selected={!settingsOpen && t.id === selectedId}
                favorite={favorites.includes(t.id)}
                onSelect={() => onSelect(t.id)}
                onToggleFavorite={() => onToggleFavorite(t.id)}
              />
            ))}
          </section>
        ))}
        {groups.length === 0 && (
          <p className="text-muted-foreground px-2">찾는 도구가 없습니다.</p>
        )}
      </nav>
      <div className="border-sidebar-border mt-auto border-t pt-2">
        <button
          type="button"
          onClick={onOpenSettings}
          className={cn(
            "hover:bg-sidebar-accent my-px flex h-8 w-full items-center gap-2 rounded-md px-2 text-left text-sm font-medium",
            settingsOpen && "bg-selection hover:bg-selection",
          )}
        >
          <IconSettings
            className={cn(
              "size-4",
              settingsOpen ? "text-primary" : "text-muted-foreground",
            )}
            stroke={1.75}
          />
          설정
        </button>
      </div>
    </aside>
  )
}
