import type { Plan } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  plan: Plan
  className?: string
}

/** 대상 확인 결과 한 줄. 건너뛴 파일 목록은 마우스를 올리면 보인다 */
export function PreviewPanel({ plan, className }: Props) {
  const dirs = [...new Set(plan.files.map((f) => f.destDir))]
  const skippedList = plan.skipped
    .map((s) => `${s.path}: ${s.reason}`)
    .join("\n")
  return (
    <p
      className={cn("min-w-0 truncate text-sm", className)}
      title={skippedList || undefined}
    >
      <span className="font-medium">
        대상 {plan.files.length}개 · 건너뜀 {plan.skipped.length}개
      </span>
      {dirs.length > 0 && (
        <span className="text-muted-foreground ml-2 text-xs">
          결과 위치: {dirs.slice(0, 2).join(", ")}
          {dirs.length > 2 && ` 외 ${dirs.length - 2}곳`}
        </span>
      )}
    </p>
  )
}
