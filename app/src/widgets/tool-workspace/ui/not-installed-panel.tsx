import type { ToolEntry } from "@/entities/tool"
import { InstallButton } from "@/features/addon-install"
import { formatBytes } from "@/shared/lib/format-bytes"

import { ToolPlaceholder } from "./tool-placeholder"

type Props = {
  tool: ToolEntry
  className?: string
}

/** 설치 안 됐거나 다시 설치해야 하는 추가 도구 */
export function NotInstalledPanel({ tool, className }: Props) {
  const reinstall = tool.status === "updateRequired"
  return (
    <ToolPlaceholder tool={tool} className={className}>
      {reinstall && (
        <p className="text-muted-foreground text-xs">
          앱이나 도구 파일이 바뀌어 다시 설치해야 합니다.
        </p>
      )}
      <InstallButton
        toolId={tool.id}
        label={reinstall ? "다시 설치" : "설치"}
      />
      {tool.size !== null && (
        <p className="text-muted-foreground text-xs">
          {formatBytes(tool.size)}
        </p>
      )}
    </ToolPlaceholder>
  )
}
