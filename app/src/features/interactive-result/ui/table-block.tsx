import { IconCopy, IconDownload } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { toCsv, toTsv } from "../lib/table-text"
import { copyText } from "../model/copy-text"
import { saveWithDialog } from "../model/save-with-dialog"

/** 화면에 그리는 최대 행 수 */
const SHOWN = 1_000

type Props = {
  columns: string[]
  rows: string[][]
  /** 있으면 [저장](CSV) 버튼, 기본 파일 이름 */
  name?: string
  className?: string
}

export function TableBlock({ columns, rows, name, className }: Props) {
  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex min-h-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      <header className="flex h-9 shrink-0 items-center justify-between pr-1.5 pl-3">
        <span className="text-stage-muted text-2xs tabular-nums">
          {rows.length.toLocaleString("ko-KR")}행
        </span>
        <div className="flex gap-1">
          <Button size="sm" onClick={() => copyText(toTsv(columns, rows))}>
            <IconCopy data-icon="inline-start" stroke={1.75} />
            복사
          </Button>
          {name && (
            <Button
              size="sm"
              onClick={() => saveWithDialog(name, toCsv(columns, rows), false)}
            >
              <IconDownload data-icon="inline-start" stroke={1.75} />
              저장
            </Button>
          )}
        </div>
      </header>
      <div className="min-h-0 overflow-auto px-3 pb-3">
        <table className="w-full border-collapse text-xs select-text">
          <thead>
            <tr className="border-stage-foreground/15 border-b">
              {columns.map((c, i) => (
                <th
                  key={i}
                  className="text-stage-muted bg-stage sticky top-0 h-7 px-2 text-left font-medium whitespace-nowrap"
                >
                  {c}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.slice(0, SHOWN).map((r, i) => (
              <tr key={i} className="border-stage-foreground/8 border-b">
                {r.map((v, j) => (
                  <td
                    key={j}
                    className="h-7 px-2 font-mono break-all whitespace-pre-wrap"
                  >
                    {v}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {rows.length > SHOWN && (
        <p className="text-stage-muted shrink-0 px-3 pb-2 text-xs">
          화면에는 앞 {SHOWN.toLocaleString("ko-KR")}행만 보입니다.
          복사·저장하면 전체가 들어갑니다.
        </p>
      )}
    </section>
  )
}
