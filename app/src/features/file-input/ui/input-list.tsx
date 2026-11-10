import type { ReactNode } from "react"
import { IconX } from "@tabler/icons-react"

import type { FileResult, PathStat } from "@/entities/tool"
import { formatBytes } from "@/shared/lib/format-bytes"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

type Props = {
  items: PathStat[]
  /** 입력 경로 → 마지막 실행 결과 */
  results?: Map<string, FileResult>
  onRemove: (path: string) => void
  onClear: () => void
  /** 이름 앞에 그릴 썸네일 (다른 기능이 채운다) */
  renderThumb?: (item: PathStat) => ReactNode
  /** 미리보기 대상으로 고른 경로 */
  selected?: string | null
  onSelect?: (path: string) => void
  disabled?: boolean
  className?: string
}

/** 경로의 마지막 이름 */
const baseName = (p: string) => p.split(/[\\/]/).pop() || p

function statusOf(
  item: PathStat,
  r?: FileResult,
): { text: string; tone?: "error" | "muted" } {
  if (!item.exists) return { text: "찾을 수 없음", tone: "error" }
  if (!r) return { text: item.isDir ? "폴더" : "대기", tone: "muted" }
  switch (r.status) {
    case "done":
      return { text: "완료" }
    case "skipped":
      return { text: `건너뜀(${r.reason})`, tone: "muted" }
    case "failed":
      return { text: r.message, tone: "error" }
    case "cancelled":
      return { text: "취소됨", tone: "muted" }
  }
}

export function InputList({
  items,
  results,
  onRemove,
  onClear,
  renderThumb,
  selected,
  onSelect,
  disabled,
  className,
}: Props) {
  if (items.length === 0) return null
  return (
    <div className={cn("flex flex-col gap-1", className)}>
      <div className="flex items-center justify-between">
        <span className="text-2xs text-muted-foreground font-medium">
          입력 {items.length}개
        </span>
        <Button variant="ghost" size="sm" onClick={onClear} disabled={disabled}>
          모두 지우기
        </Button>
      </div>
      <div className="max-h-64 overflow-auto rounded-md border">
        <table className="w-full table-fixed border-collapse text-sm">
          <thead className="bg-surface-header sticky top-0">
            <tr className="text-2xs text-muted-foreground h-7 border-b text-left">
              <th className="px-2 font-medium">이름</th>
              <th className="w-20 px-2 text-right font-medium">크기</th>
              <th className="w-28 px-2 font-medium">상태</th>
              <th className="w-8" />
            </tr>
          </thead>
          <tbody>
            {items.map((item) => {
              const s = statusOf(item, results?.get(item.path))
              return (
                <tr
                  key={item.path}
                  onClick={() => onSelect?.(item.path)}
                  aria-selected={selected === item.path}
                  className={cn(
                    "odd:bg-muted/60 hover:bg-accent h-10",
                    onSelect && "cursor-default",
                    selected === item.path &&
                      "bg-selection odd:bg-selection hover:bg-selection",
                  )}
                >
                  <td className="px-2" title={item.path}>
                    <span className="flex min-w-0 items-center gap-2">
                      {renderThumb?.(item)}
                      <span className="truncate">{baseName(item.path)}</span>
                    </span>
                  </td>
                  <td className="text-muted-foreground px-2 text-right tabular-nums">
                    {item.isDir ? "폴더" : formatBytes(item.size)}
                  </td>
                  <td
                    className={cn(
                      "truncate px-2",
                      s.tone === "error" && "text-destructive",
                      s.tone === "muted" && "text-muted-foreground",
                    )}
                    title={s.text}
                  >
                    {s.text}
                  </td>
                  <td className="px-1">
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      aria-label="목록에서 빼기"
                      onClick={(e) => {
                        e.stopPropagation()
                        onRemove(item.path)
                      }}
                      disabled={disabled}
                    >
                      <IconX className="size-3.5" stroke={1.75} />
                    </Button>
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </div>
  )
}
