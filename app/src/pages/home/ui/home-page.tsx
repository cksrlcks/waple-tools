import { useState } from "react"

import { ToolList, useTools } from "@/entities/tool"
import { ToolRunner } from "@/features/tool-run"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { UiCheckPanel } from "@/widgets/ui-check"

export function HomePage() {
  const tools = useTools()
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const selected =
    tools.data?.find((t) => t.meta.id === selectedId) ?? tools.data?.[0]

  return (
    <div className="bg-background text-foreground flex h-screen flex-col overflow-hidden text-sm">
      {tools.isError && <ErrorBanner message={toUserMessage(tools.error)} />}
      <div className="flex min-h-0 flex-1">
        <aside className="border-border bg-sidebar flex w-60 flex-col overflow-y-auto border-r p-2">
          {tools.isPending && (
            <p className="text-muted-foreground px-2">불러오는 중…</p>
          )}
          {tools.data && (
            <ToolList
              tools={tools.data}
              selectedId={selected?.meta.id ?? null}
              onSelect={setSelectedId}
            />
          )}
        </aside>
        <main className="flex flex-1 flex-col gap-4 overflow-auto p-4">
          {selected && <ToolRunner key={selected.meta.id} tool={selected} />}
          <UiCheckPanel />
        </main>
      </div>
    </div>
  )
}
