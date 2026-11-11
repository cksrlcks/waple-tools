import {
  IconAlertCircleFilled,
  IconArrowBackUp,
  IconCircleCheckFilled,
  IconFolderOpen,
} from "@tabler/icons-react"
import { revealItemInDir } from "@tauri-apps/plugin-opener"

import type { JobReport, UndoItem } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { summarizeReport } from "../lib/summarize-report"
import { summarizeUndo } from "../lib/summarize-undo"

type Props = {
  report: JobReport
  undoItems?: UndoItem[]
  onUndo: () => void
  undoing: boolean
  /** 결과 세트 폴더를 만드는 도구 */
  generator?: boolean
  className?: string
}

/** 실행 결과 한 줄: 상태 · 용량 변화 · 폴더 열기 · 되돌리기 */
export function ResultReport({
  report,
  undoItems,
  onUndo,
  undoing,
  generator = false,
  className,
}: Props) {
  const s = summarizeReport(report, generator)
  // 폴더를 열고 첫 결과 파일(세트 도구는 세트 폴더)을 선택해 보여 준다
  const firstOutput = generator
    ? s.openDir
    : report.files.find((f) => f.status === "done")?.outputs[0]
  const skippedUndo =
    undoItems
      ?.flatMap((i) =>
        i.status === "skipped" ? [`${i.path}: ${i.reason}`] : [],
      )
      .join("\n") ?? ""
  const ok = s.failed === 0
  const StatusIcon = ok ? IconCircleCheckFilled : IconAlertCircleFilled
  return (
    <div className={cn("flex min-w-0 items-center gap-3", className)}>
      <StatusIcon
        className={cn(
          "size-5 shrink-0",
          ok ? "text-success" : "text-destructive",
        )}
      />
      <p className="text-sm font-medium whitespace-nowrap">
        성공 {s.done} ·{" "}
        <span className={cn(!ok && "text-destructive")}>실패 {s.failed}</span> ·
        건너뜀 {s.skipped}
        {s.cancelled > 0 && ` · 취소 ${s.cancelled}`}
      </p>
      {s.change && (
        <span className="text-muted-foreground truncate text-xs tabular-nums">
          {s.change}
        </span>
      )}
      <div className="flex shrink-0 items-center gap-1.5">
        {firstOutput && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => revealItemInDir(firstOutput).catch(() => {})}
          >
            <IconFolderOpen stroke={1.75} />
            폴더 열기
          </Button>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={onUndo}
          disabled={undoing || !!undoItems || s.done === 0}
        >
          <IconArrowBackUp stroke={1.75} />
          되돌리기
        </Button>
      </div>
      {undoItems && (
        <span
          className="text-muted-foreground truncate text-xs"
          title={skippedUndo || undefined}
        >
          {summarizeUndo(undoItems)}
        </span>
      )}
    </div>
  )
}
