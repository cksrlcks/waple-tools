import { useEffect, useState } from "react"

import { useSaveSettings, useSettings } from "@/entities/settings"
import { CATEGORY_LABELS, useTools } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { AppSidebar } from "@/widgets/app-sidebar"
import { categoryTabs, CategoryTabs } from "@/widgets/category-tabs"
import { SettingsDialog } from "@/widgets/settings-panel"
import { TitleBar } from "@/widgets/title-bar"
import { ToolWorkspace } from "@/widgets/tool-workspace"

import { ComingSoon } from "./coming-soon"

export function HomePage() {
  const tools = useTools()
  const settings = useSettings()
  const save = useSaveSettings()
  const [tab, setTab] = useState("image")
  const [toolId, setToolId] = useState<string | null>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)
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
  const selectedId = toolId ?? fallbackId
  const selected = list.find((t) => t.id === selectedId)

  const toggleFavorite = (id: string) => {
    if (!settings.data) return
    const next = favorites.includes(id)
      ? favorites.filter((f) => f !== id)
      : [...favorites, id]
    save.mutate({ ...settings.data, favorites: next })
  }

  // 탭별 강조색: <html>에 붙여야 포털(대화상자·메뉴)까지 같은 색이 된다
  useEffect(() => {
    document.documentElement.dataset.category = tab
  }, [tab])

  const changeTab = (key: string) => {
    setTab(key)
    setToolId(null)
  }

  const error = tools.isError
    ? toUserMessage(tools.error)
    : settings.isError
      ? toUserMessage(settings.error)
      : null

  return (
    <div className="bg-background text-foreground flex h-screen flex-col overflow-hidden text-sm">
      <TitleBar
        onOpenSettings={() => setSettingsOpen(true)}
        onHome={() => changeTab("image")}
        locked={busy}
      >
        <CategoryTabs
          tabs={categoryTabs(import.meta.env.DEV)}
          value={tab}
          onChange={changeTab}
          locked={busy}
        />
      </TitleBar>
      {error && <ErrorBanner message={error} />}
      <div className="flex min-h-0 flex-1">
        {(tools.isPending || list.length > 0) && (
          <AppSidebar
            tools={list}
            favorites={favorites}
            selectedId={selectedId}
            onSelect={setToolId}
            onToggleFavorite={toggleFavorite}
            locked={busy}
          />
        )}
        <main className="flex min-w-0 flex-1">
          {tools.isPending || settings.isPending ? (
            <p className="text-muted-foreground m-auto">불러오는 중…</p>
          ) : selected && settings.data ? (
            <ToolWorkspace
              key={selected.id}
              tool={selected}
              settings={settings.data}
              onBusyChange={setBusy}
            />
          ) : (
            <ComingSoon label={CATEGORY_LABELS[tab] ?? tab} />
          )}
        </main>
      </div>
      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
    </div>
  )
}
