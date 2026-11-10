import { useState } from "react"

import { useSaveSettings, useSettings } from "@/entities/settings"
import { useTools } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { AppSidebar } from "@/widgets/app-sidebar"
import { categoryTabs, CategoryTabs } from "@/widgets/category-tabs"
import { SettingsPanel } from "@/widgets/settings-panel"
import { ToolWorkspace } from "@/widgets/tool-workspace"

type View = { type: "tool"; id: string | null } | { type: "settings" }

export function HomePage() {
  const tools = useTools()
  const settings = useSettings()
  const save = useSaveSettings()
  const [tab, setTab] = useState("image")
  const [view, setView] = useState<View>({ type: "tool", id: null })
  // 실행·되돌리기 중에는 다른 화면으로 가지 못하게 한다 (작업 화면이 사라지면 취소·되돌리기를 못 함)
  const [busy, setBusy] = useState(false)

  const all = tools.data ?? []
  const list = all.filter((t) => t.category === tab)
  const favorites = settings.data?.favorites ?? []
  // 처음엔 이 탭의 즐겨찾기 첫 도구, 없으면 첫 설치된 도구
  const fallbackId =
    favorites.find((id) => list.some((t) => t.id === id)) ??
    list.find((t) => t.status === "builtin")?.id ??
    list[0]?.id ??
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

  const changeTab = (key: string) => {
    setTab(key)
    setView({ type: "tool", id: null })
  }

  const error = tools.isError
    ? toUserMessage(tools.error)
    : settings.isError
      ? toUserMessage(settings.error)
      : null

  return (
    <div className="bg-background text-foreground flex h-screen flex-col overflow-hidden text-sm">
      <CategoryTabs
        tabs={[
          ...categoryTabs(all, import.meta.env.DEV),
          { key: "settings", label: "설정", disabled: false, end: true },
        ]}
        value={view.type === "settings" ? "settings" : tab}
        onChange={(key) =>
          key === "settings" ? setView({ type: "settings" }) : changeTab(key)
        }
        locked={busy}
      />
      {error && <ErrorBanner message={error} />}
      <div className="flex min-h-0 flex-1">
        <AppSidebar
          tools={list}
          favorites={favorites}
          selectedId={selectedId}
          onSelect={(id) => setView({ type: "tool", id })}
          onToggleFavorite={toggleFavorite}
          locked={busy}
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
              onBusyChange={setBusy}
            />
          ) : (
            <p className="text-muted-foreground flex h-full items-center justify-center">
              이 탭에는 아직 도구가 없습니다.
            </p>
          )}
        </main>
      </div>
    </div>
  )
}
