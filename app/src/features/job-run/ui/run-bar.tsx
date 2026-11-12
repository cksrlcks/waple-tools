import type { ReactNode } from "react"
import { IconPlayerPlayFilled } from "@tabler/icons-react"
import { toast } from "sonner"

import type { Progress as JobProgress } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Progress } from "@/shared/ui/progress"

type Props = {
  /** 왼쪽 상태 자리: 결과 요약·대상 확인 요약 (실행 중엔 진행 막대가 대신 나온다) */
  children?: ReactNode
  /** 없으면 "대상 확인" 버튼을 그리지 않는다 (파일명 변경처럼 미리보기가 늘 보이는 도구) */
  onPreview?: () => void
  onRun: () => void
  /** 실행 버튼 문구 (기본 "실행") */
  runLabel?: string
  onCancel: () => void
  running: boolean
  previewing: boolean
  progress: JobProgress | null
  /** 취소를 눌렀고 멈추기를 기다리는 중 */
  cancelling?: boolean
  /** 누를 것이 없을 때(파일 없음) 미리보기·실행 비활성 */
  disabled: boolean
  /** 실행할 수 없는 이유. 누르면 토스트로 알리고 멈춘다 */
  blockedReason: string | null
  className?: string
}

/** 작업 영역 맨 아래 가로 막대: 왼쪽은 상태, 오른쪽은 대상 확인·실행 */
export function RunBar({
  children,
  onPreview,
  onRun,
  runLabel = "실행",
  onCancel,
  running,
  previewing,
  progress,
  cancelling = false,
  disabled,
  blockedReason,
  className,
}: Props) {
  const guard = (action: () => void) => () => {
    if (blockedReason) toast.error(blockedReason)
    else action()
  }
  const percent = progress ? progress.ratio * 100 : 0
  // 파일이 여럿이면 "3/10", 하나면 그 파일 안의 진행률 (예: PDF 쪽수)
  const label = cancelling
    ? "취소 중…"
    : !progress
      ? "준비 중"
      : progress.total > 1
        ? `${progress.done}/${progress.total}`
        : `${Math.round(percent)}%`
  return (
    <div className={cn("flex items-center gap-4", className)}>
      <div className="flex min-w-0 flex-1 items-center">
        {running ? (
          <div className="flex w-full max-w-md items-center gap-2.5">
            <Progress value={percent} className="flex-1" />
            <span className="text-muted-foreground text-xs tabular-nums">
              {label}
            </span>
          </div>
        ) : (
          children
        )}
      </div>
      <div className="flex w-72 shrink-0 items-center gap-2">
        {onPreview && (
          <Button
            size="lg"
            variant="outline"
            onClick={guard(onPreview)}
            disabled={disabled || previewing || running}
          >
            대상 확인
          </Button>
        )}
        {running ? (
          <Button
            size="lg"
            variant="outline"
            className="flex-1"
            onClick={onCancel}
            disabled={cancelling}
          >
            취소
          </Button>
        ) : (
          <Button
            size="lg"
            className="flex-1 font-medium"
            onClick={guard(onRun)}
            disabled={disabled}
          >
            <IconPlayerPlayFilled className="size-3.5" />
            {runLabel}
          </Button>
        )}
      </div>
    </div>
  )
}
