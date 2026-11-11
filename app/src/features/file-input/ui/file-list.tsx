import type { ReactNode } from "react"

import type { FileResult, PathStat } from "@/entities/tool"
import { formatBytes } from "@/shared/lib/format-bytes"
import { cn } from "@/shared/lib/utils"

import { FileRow } from "./file-row"

type Props = {
  items: PathStat[]
  /** 입력 경로 → 마지막 실행 결과 */
  results?: Map<string, FileResult>
  onRemove: (path: string) => void
  onClear: () => void
  /** 줄 앞 아이콘·썸네일 (다른 기능이 채운다) */
  renderThumb?: (item: PathStat) => ReactNode
  /** 줄마다 상태 앞에 둘 것 (파일마다 고르는 옵션) */
  renderExtra?: (item: PathStat) => ReactNode
  /** 목록 끝에 붙일 것 (추가 버튼) */
  trailing?: ReactNode
  disabled?: boolean
  className?: string
}

/** 미리보기가 없는 도구의 작업대: 넣은 파일을 세로 목록으로 */
export function FileList({
  items,
  results,
  onRemove,
  onClear,
  renderThumb,
  renderExtra,
  trailing,
  disabled,
  className,
}: Props) {
  const total = items.reduce((n, i) => n + (i.isDir ? 0 : i.size), 0)
  return (
    <section className={cn("flex min-h-0 flex-col", className)}>
      <div className="text-stage-muted flex h-11 shrink-0 items-center gap-3 px-4 text-xs">
        <span className="text-stage-foreground text-sm font-medium tabular-nums">
          {items.length}개 · {formatBytes(total)}
        </span>
        <button
          type="button"
          onClick={onClear}
          disabled={disabled}
          className="hover:text-stage-foreground ml-auto disabled:opacity-50"
        >
          모두 비우기
        </button>
      </div>
      <ul className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-3">
        {items.map((item) => (
          <FileRow
            key={item.path}
            item={item}
            result={results?.get(item.path)}
            thumb={renderThumb?.(item)}
            extra={renderExtra?.(item)}
            onRemove={() => onRemove(item.path)}
            disabled={disabled}
          />
        ))}
        {trailing && <li className="px-2 pt-2">{trailing}</li>}
      </ul>
    </section>
  )
}
