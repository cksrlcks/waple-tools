import type { JobReport } from "@/entities/tool"
import { formatChange } from "@/shared/lib/format-bytes"

export type ReportSummary = {
  done: number
  failed: number
  /** 파일 단위 건너뜀 + 계획 단계 제외 */
  skipped: number
  cancelled: number
  sizeBefore: number
  sizeAfter: number
  change: string | null
  /** "폴더 열기"로 보여 줄 첫 결과의 폴더 */
  openDir: string | null
}

function parentDir(p: string): string {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"))
  return i > 0 ? p.slice(0, i) : p
}

export function summarizeReport(report: JobReport): ReportSummary {
  const s: ReportSummary = {
    done: 0,
    failed: 0,
    skipped: report.skipped.length,
    cancelled: 0,
    sizeBefore: 0,
    sizeAfter: 0,
    change: null,
    openDir: null,
  }
  for (const f of report.files) {
    if (f.status === "done") {
      s.done++
      s.sizeBefore += f.sizeBefore
      s.sizeAfter += f.sizeAfter
      if (!s.openDir && f.outputs[0]) s.openDir = parentDir(f.outputs[0])
    } else if (f.status === "failed") s.failed++
    else if (f.status === "skipped") s.skipped++
    else s.cancelled++
  }
  if (s.done > 0) s.change = formatChange(s.sizeBefore, s.sizeAfter)
  return s
}
