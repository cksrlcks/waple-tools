import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Progress } from "@/shared/ui/progress"

import { useInstallTool } from "../api/use-install-tool"
import { downloadPercent } from "../lib/download-percent"

type Props = {
  toolId: string
  label?: string
  className?: string
}

/** 설치 버튼. 누르면 받는 동안 진행률 막대로 바뀐다 */
export function InstallButton({ toolId, label = "설치", className }: Props) {
  const { install, installing, progress } = useInstallTool(toolId)
  const pct = downloadPercent(progress)
  if (installing)
    return (
      <div className={cn("flex w-48 flex-col gap-1.5", className)}>
        <Progress value={pct} aria-label="받는 중" />
        <span className="text-muted-foreground text-xs tabular-nums">
          {pct === null ? "받는 중…" : `받는 중… ${pct}%`}
        </span>
      </div>
    )
  return (
    <Button size="lg" onClick={install} className={className}>
      {label}
    </Button>
  )
}
