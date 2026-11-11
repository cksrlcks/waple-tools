import type { Settings } from "@/entities/settings"
import type { ToolEntry } from "@/entities/tool"

import { FileToolWorkspace } from "./file-tool-workspace"
import { InteractiveToolWorkspace } from "./interactive-tool-workspace"
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
  if (tool.status === "notInstalled" || tool.status === "updateRequired")
    return <NotInstalledPanel tool={tool} className={className} />
  if (tool.kind === "interactive")
    return (
      <InteractiveToolWorkspace
        tool={tool}
        settings={settings}
        className={className}
      />
    )
  return (
    <FileToolWorkspace
      tool={tool}
      settings={settings}
      onBusyChange={onBusyChange}
      className={className}
    />
  )
}
