import type { ToolEntry } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { useInstallTool } from "../api/use-install-tool"
import { useUninstallTool } from "../api/use-uninstall-tool"
import { downloadPercent } from "../lib/download-percent"

type Props = {
  tool: ToolEntry
  className?: string
}

/** 설정의 추가 도구 한 줄 오른쪽: 업데이트(또는 다시 설치) · 제거 */
export function AddonActions({ tool, className }: Props) {
  const { install, installing, progress } = useInstallTool(tool.id)
  const { uninstall, uninstalling } = useUninstallTool(tool.id)
  const pct = downloadPercent(progress)
  const updateLabel =
    tool.status === "updateRequired"
      ? "다시 설치"
      : tool.status === "updateAvailable"
        ? "업데이트"
        : null
  return (
    <div className={cn("flex shrink-0 items-center gap-2", className)}>
      {installing ? (
        <span className="text-muted-foreground text-xs tabular-nums">
          {pct === null ? "받는 중…" : `받는 중… ${pct}%`}
        </span>
      ) : (
        updateLabel && (
          <Button size="sm" onClick={install} disabled={uninstalling}>
            {updateLabel}
          </Button>
        )
      )}
      <Button
        size="sm"
        variant="outline"
        onClick={uninstall}
        disabled={installing || uninstalling}
      >
        제거
      </Button>
    </div>
  )
}
