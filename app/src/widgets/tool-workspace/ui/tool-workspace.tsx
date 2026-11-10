import type { Settings } from "@/entities/settings"
import type { ToolEntry } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { FileToolWorkspace } from "./file-tool-workspace"
import { NotInstalledPanel } from "./not-installed-panel"

type Props = {
  tool: ToolEntry
  settings: Settings
  onBusyChange?: (busy: boolean) => void
  className?: string
}

export function ToolWorkspace({
  tool,
  settings,
  onBusyChange,
  className,
}: Props) {
  if (tool.status === "notInstalled")
    return <NotInstalledPanel tool={tool} className={className} />
  if (tool.kind === "interactive") {
    return (
      <section className={cn("flex flex-col gap-1", className)}>
        <h1 className="text-base font-semibold">{tool.name}</h1>
        <p className="text-muted-foreground">이 도구 화면은 준비 중입니다.</p>
      </section>
    )
  }
  return (
    <FileToolWorkspace
      tool={tool}
      settings={settings}
      onBusyChange={onBusyChange}
      className={className}
    />
  )
}
