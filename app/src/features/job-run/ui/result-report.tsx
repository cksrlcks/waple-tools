import { IconArrowBackUp, IconFolderOpen } from "@tabler/icons-react"
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
  className?: string
}

export function ResultReport({
  report,
  undoItems,
  onUndo,
  undoing,
  className,
}: Props) {
  const s = summarizeReport(report)
  // 폴더를 열고 첫 결과 파일을 선택해 보여 준다
  const firstOutput = report.files.find((f) => f.status === "done")?.outputs[0]
  const skippedUndo =
    undoItems?.flatMap((i) => (i.status === "skipped" ? [i] : [])) ?? []
  return (
    <section
      className={cn(
        "bg-card flex flex-col gap-2.5 rounded-lg border p-3",
        className,
      )}
    >
      <p className="text-sm font-medium">
        성공 {s.done} · 실패 {s.failed} · 건너뜀 {s.skipped}
        {s.cancelled > 0 && ` · 취소 ${s.cancelled}`}
      </p>
      {s.change && (
        <p className="text-muted-foreground text-xs tabular-nums">{s.change}</p>
      )}
      <div className="flex items-center gap-2">
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
        {undoItems && (
          <span className="text-muted-foreground text-xs">
            {summarizeUndo(undoItems)}
          </span>
        )}
      </div>
      {skippedUndo.length > 0 && (
        <ul className="text-muted-foreground text-xs">
          {skippedUndo.map((i) => (
            <li key={i.path} className="truncate" title={i.path}>
              {i.path}: {i.reason}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
