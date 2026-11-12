import type { Settings } from "@/entities/settings"
import type { ToolEntry } from "@/entities/tool"
import { AddonUpdateBar } from "@/features/addon-install"

import { FileToolWorkspace } from "./file-tool-workspace"
import { InteractiveToolWorkspace } from "./interactive-tool-workspace"
import { NotInstalledPanel } from "./not-installed-panel"
import { PlannedPanel } from "./planned-panel"
import { RenameToolWorkspace } from "./rename-tool-workspace"

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
  if (tool.status === "planned")
    return <PlannedPanel tool={tool} className={className} />
  if (tool.status === "notInstalled" || tool.status === "updateRequired")
    return <NotInstalledPanel tool={tool} className={className} />
  const workspace =
    tool.kind === "interactive" ? (
      <InteractiveToolWorkspace
        tool={tool}
        settings={settings}
        className={className}
      />
    ) : tool.kind === "rename" ? (
      <RenameToolWorkspace
        tool={tool}
        settings={settings}
        onBusyChange={onBusyChange}
        className={className}
      />
    ) : (
      <FileToolWorkspace
        tool={tool}
        settings={settings}
        onBusyChange={onBusyChange}
        className={className}
      />
    )
  if (tool.status !== "updateAvailable") return workspace
  // 새 버전이 있으면 맨 위에 업데이트 막대 (사이드바 "업데이트" 배지를 눌러 들어온 곳)
  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <AddonUpdateBar tool={tool} />
      <div className="flex min-h-0 flex-1">{workspace}</div>
    </div>
  )
}
