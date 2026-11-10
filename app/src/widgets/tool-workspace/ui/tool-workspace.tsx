import type { Settings } from "@/entities/settings"
import type { ToolEntry } from "@/entities/tool"

import { FileToolWorkspace } from "./file-tool-workspace"
import { NotInstalledPanel } from "./not-installed-panel"
import { ToolPlaceholder } from "./tool-placeholder"

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
      <ToolPlaceholder tool={tool} className={className}>
        <p className="text-muted-foreground text-xs">
          이 도구 화면은 준비 중입니다.
        </p>
      </ToolPlaceholder>
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
