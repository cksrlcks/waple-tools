import { IconPlayerPlayFilled } from "@tabler/icons-react"

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

/** 옵션 패널 맨 아래에 붙는 실행 막대 */
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
    <div className={cn("flex flex-col gap-2.5", className)}>
      {(running || progress) && (
        <div className="flex items-center gap-2.5">
          <Progress value={percent} className="flex-1" />
          <span className="text-muted-foreground text-xs tabular-nums">
            {progress ? `${progress.done}/${progress.total}` : "준비 중"}
          </span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <Button
          size="lg"
          variant="outline"
          onClick={onPreview}
          disabled={blocked || previewing}
        >
          대상 확인
        </Button>
        {running ? (
          <Button
            size="lg"
            variant="outline"
            className="flex-1"
            onClick={onCancel}
          >
            취소
          </Button>
        ) : (
          <Button
            size="lg"
            className="flex-1 font-medium"
            onClick={onRun}
            disabled={blocked}
          >
            <IconPlayerPlayFilled className="size-3.5" />
            실행
          </Button>
        )}
      </div>
      {blocked && !running && (
        <p className="text-muted-foreground text-xs">{disabledReason}</p>
      )}
    </div>
  )
}
