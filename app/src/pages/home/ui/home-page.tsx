import { useState } from "react"

import { useSaveSettings, useSettings } from "@/entities/settings"
import { useTools } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { AppSidebar } from "@/widgets/app-sidebar"
import { SettingsPanel } from "@/widgets/settings-panel"
import { ToolWorkspace } from "@/widgets/tool-workspace"

type View = { type: "tool"; id: string | null } | { type: "settings" }

export function HomePage() {
  const tools = useTools()
  const settings = useSettings()
  const save = useSaveSettings()
  const [view, setView] = useState<View>({ type: "tool", id: null })

  const list = tools.data ?? []
  const favorites = settings.data?.favorites ?? []
  // 처음엔 즐겨찾기 첫 도구, 없으면 첫 설치된 도구
  const fallbackId =
    favorites.find((id) => list.some((t) => t.id === id)) ??
    list.find((t) => t.status === "builtin")?.id ??
    null
  const selectedId = view.type === "tool" ? (view.id ?? fallbackId) : null
  const selected = list.find((t) => t.id === selectedId)

  const toggleFavorite = (id: string) => {
    if (!settings.data) return
    const next = favorites.includes(id)
      ? favorites.filter((f) => f !== id)
      : [...favorites, id]
    save.mutate({ ...settings.data, favorites: next })
  }

  const error = tools.isError
    ? toUserMessage(tools.error)
    : settings.isError
      ? toUserMessage(settings.error)
      : null

  return (
    <div className="bg-background text-foreground flex h-screen flex-col overflow-hidden text-sm">
      {error && <ErrorBanner message={error} />}
      <div className="flex min-h-0 flex-1">
        <AppSidebar
          tools={list}
          favorites={favorites}
          selectedId={selectedId}
          settingsOpen={view.type === "settings"}
          onSelect={(id) => setView({ type: "tool", id })}
          onOpenSettings={() => setView({ type: "settings" })}
          onToggleFavorite={toggleFavorite}
        />
        <main className="flex min-w-0 flex-1 flex-col overflow-auto p-4">
          {view.type === "settings" ? (
            <SettingsPanel />
          ) : tools.isPending || settings.isPending ? (
            <p className="text-muted-foreground">불러오는 중…</p>
          ) : selected && settings.data ? (
            <ToolWorkspace
              key={selected.id}
              tool={selected}
              settings={settings.data}
            />
          ) : (
            <p className="text-muted-foreground flex h-full items-center justify-center">
              왼쪽에서 도구를 고르세요.
            </p>
          )}
        </main>
      </div>
    </div>
  )
}
