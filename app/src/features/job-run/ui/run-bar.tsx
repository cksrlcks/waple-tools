import type { Progress as JobProgress } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Progress } from "@/shared/ui/progress"

type Props = {
  onPreview: () => void
  onRun: () => void
  onCancel: () => void
  running: boolean
  previewing: boolean
  progress: JobProgress | null
  /** 실행할 수 없는 이유. 있으면 미리보기·실행 비활성 */
  disabledReason: string | null
  className?: string
}

export function RunBar({
  onPreview,
  onRun,
  onCancel,
  running,
  previewing,
  progress,
  disabledReason,
  className,
}: Props) {
  const blocked = disabledReason !== null
  const percent = progress?.total ? (progress.done / progress.total) * 100 : 0
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          onClick={onPreview}
          disabled={blocked || previewing}
        >
          미리보기
        </Button>
        <Button onClick={onRun} disabled={blocked}>
          실행
        </Button>
        {running && (
          <Button variant="outline" onClick={onCancel}>
            취소
          </Button>
        )}
        {blocked && !running && (
          <span className="text-muted-foreground text-xs">
            {disabledReason}
          </span>
        )}
      </div>
      {(running || progress) && (
        <div className="flex items-center gap-2">
          <Progress value={percent} className="flex-1" />
          <span className="text-muted-foreground text-xs tabular-nums">
            {progress ? `${progress.done}/${progress.total}` : "준비 중"}
          </span>
        </div>
      )}
    </div>
  )
}
