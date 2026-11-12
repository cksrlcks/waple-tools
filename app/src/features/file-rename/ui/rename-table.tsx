import type { ReactNode } from "react"

import type { RenameRow } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { RenameTableRow } from "./rename-table-row"

type Props = {
  /** 미리보기가 아직 없으면 undefined */
  rows: RenameRow[] | undefined
  /** 넣은 경로 (이 경로인 줄만 목록에서 뺄 수 있다) */
  inputs: string[]
  onRemove: (path: string) => void
  onClear: () => void
  /** 표 위 요약 (예: 바뀜 3 · 충돌 1) */
  summary?: ReactNode
  /** 표 끝에 붙일 것 (추가 버튼) */
  trailing?: ReactNode
  disabled?: boolean
  className?: string
}

/** 파일명 일괄 변경 작업대: 변경 전/후 미리보기 표 */
export function RenameTable({
  rows,
  inputs,
  onRemove,
  onClear,
  summary,
  trailing,
  disabled,
  className,
}: Props) {
  return (
    <section className={cn("flex min-h-0 flex-col", className)}>
      <div className="text-stage-muted flex h-11 shrink-0 items-center gap-3 px-4 text-xs">
        <span className="text-stage-foreground text-sm font-medium tabular-nums">
          {rows?.length ?? 0}개
        </span>
        {summary}
        <button
          type="button"
          onClick={onClear}
          disabled={disabled}
          className="hover:text-stage-foreground ml-auto disabled:opacity-50"
        >
          모두 비우기
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        {rows ? (
          <table className="w-full table-fixed border-collapse text-sm">
            <thead className="bg-stage sticky top-0">
              <tr className="border-stage-foreground/10 text-2xs text-stage-muted h-7 border-b text-left">
                <th className="px-3 font-medium">현재 이름</th>
                <th className="px-3 font-medium">새 이름</th>
                <th className="w-48 px-3 font-medium">상태</th>
                <th className="w-8" />
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <RenameTableRow
                  key={r.from}
                  row={r}
                  onRemove={
                    inputs.includes(r.from) ? () => onRemove(r.from) : undefined
                  }
                  disabled={disabled}
                />
              ))}
            </tbody>
          </table>
        ) : (
          <p className="text-stage-muted flex h-24 items-center justify-center text-sm">
            미리보기를 만드는 중…
          </p>
        )}
        {trailing && <div className="px-2 pt-2">{trailing}</div>}
      </div>
    </section>
  )
}
