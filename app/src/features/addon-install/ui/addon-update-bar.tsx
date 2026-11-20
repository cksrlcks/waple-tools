import type { ToolEntry } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { useInstallTool } from "../api/use-install-tool"
import { progressText } from "../lib/progress-text"

type Props = {
  tool: ToolEntry
  className?: string
}

/** 새 버전이 있는 추가 도구의 작업 화면 맨 위 막대: 버전 안내 + 업데이트 */
export function AddonUpdateBar({ tool, className }: Props) {
  const { install, installing, progress } = useInstallTool(tool.id)
  return (
    <div
      className={cn(
        "bg-primary/10 flex items-center gap-3 border-b px-4 py-2 text-xs",
        className,
      )}
    >
      <span className="flex-1">
        새 버전이 있습니다 ({tool.version} → {tool.latest})
      </span>
      {installing ? (
        <span className="text-muted-foreground tabular-nums">
          {progressText(progress)}
        </span>
      ) : (
        <Button size="sm" onClick={install}>
          업데이트
        </Button>
      )}
    </div>
  )
}
