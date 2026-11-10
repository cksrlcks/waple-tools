import { useState } from "react"

import { ToolRow, type ToolEntry } from "@/entities/tool"
import { updateLabel, type useAppUpdate } from "@/features/app-update"
import {
  filterTools,
  groupTools,
  ToolSearchInput,
} from "@/features/tool-search"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import pkg from "../../../../package.json"

type Props = {
  tools: ToolEntry[]
  favorites: string[]
  /** 설정 화면이면 null */
  selectedId: string | null
  onSelect: (id: string) => void
  onToggleFavorite: (id: string) => void
  /** 작업 중이면 다른 도구로 이동하지 못한다 */
  locked?: boolean
  /** 앱 업데이트 상태와 동작 (앱 전체에서 한 번만 확인하므로 위에서 받는다) */
  updater: ReturnType<typeof useAppUpdate>
  className?: string
}

export function AppSidebar({
  tools,
  favorites,
  selectedId,
  onSelect,
  onToggleFavorite,
  locked = false,
  updater,
  className,
}: Props) {
  const { status } = updater
  const available = status.kind === "available"
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
      {/* 맨 아래: 버전 · 업데이트 확인 */}
      <div className="mt-auto flex items-center justify-between pt-2">
        <span className="text-muted-foreground px-2 text-xs tabular-nums">
          v{pkg.version}
        </span>
        {/* 새 버전이 있으면 강조 버튼. 작업 중에는 설치하지 않는다 */}
        <Button
          variant={available ? "default" : "ghost"}
          size="sm"
          className={cn("text-xs", !available && "text-muted-foreground")}
          onClick={available ? updater.install : updater.check}
          disabled={
            status.kind === "checking" ||
            status.kind === "installing" ||
            (available && locked)
          }
          title={
            available && locked
              ? "작업이 끝난 뒤 설치할 수 있습니다."
              : undefined
          }
        >
          {updateLabel(status)}
        </Button>
      </div>
    </aside>
  )
}
