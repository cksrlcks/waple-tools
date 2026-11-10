import type { ReactNode } from "react"
import {
  IconAlertCircleFilled,
  IconCircleCheckFilled,
  IconX,
} from "@tabler/icons-react"

import type { FileResult, PathStat } from "@/entities/tool"
import { baseName } from "@/shared/lib/base-name"
import { formatBytes } from "@/shared/lib/format-bytes"
import { cn } from "@/shared/lib/utils"

type Props = {
  items: PathStat[]
  /** 입력 경로 → 마지막 실행 결과 */
  results?: Map<string, FileResult>
  onRemove: (path: string) => void
  onClear: () => void
  /** 카드 위쪽 그림 (다른 기능이 채운다) */
  renderThumb?: (item: PathStat) => ReactNode
  /** 미리보기 대상으로 고른 경로 */
  selected?: string | null
  onSelect?: (path: string) => void
  /** 띠 끝에 붙일 것 (추가 버튼) */
  trailing?: ReactNode
  disabled?: boolean
  className?: string
}

function statusOf(
  item: PathStat,
  r?: FileResult,
): { text: string; tone?: "error" | "done" } {
  if (!item.exists) return { text: "찾을 수 없음", tone: "error" }
  if (!r) return { text: item.isDir ? "폴더" : formatBytes(item.size) }
  switch (r.status) {
    case "done":
      return { text: "완료", tone: "done" }
    case "skipped":
      return { text: `건너뜀(${r.reason})` }
    case "failed":
      return { text: r.message, tone: "error" }
    case "cancelled":
      return { text: "취소됨" }
  }
}

/** 작업대 아래 필름 띠: 넣은 파일 카드를 가로로 늘어놓는다. 누르면 미리보기 대상이 된다 */
export function InputList({
  items,
  results,
  onRemove,
  onClear,
  renderThumb,
  selected,
  onSelect,
  trailing,
  disabled,
  className,
}: Props) {
  if (items.length === 0) return null
  const total = items.reduce((n, i) => n + (i.isDir ? 0 : i.size), 0)
  return (
    <div
      className={cn(
        "border-stage-foreground/10 flex shrink-0 flex-col gap-2 border-t px-4 pt-2.5 pb-3",
        className,
      )}
    >
      <div className="text-stage-muted flex items-center justify-between text-xs">
        <span className="tabular-nums">
          {items.length}개 · {formatBytes(total)}
        </span>
        <button
          type="button"
          onClick={onClear}
          disabled={disabled}
          className="hover:text-stage-foreground disabled:opacity-50"
        >
          모두 비우기
        </button>
      </div>
      <ul className="flex gap-2 overflow-x-auto p-0.5 pb-1">
        {items.map((item) => {
          const s = statusOf(item, results?.get(item.path))
          const active = selected === item.path
          return (
            <li key={item.path} className="group relative w-24 shrink-0">
              <button
                type="button"
                onClick={() => onSelect?.(item.path)}
                aria-pressed={active}
                title={item.path}
                className="focus-visible:ring-primary block w-full rounded-lg text-left outline-none focus-visible:ring-2"
              >
                <span
                  className={cn(
                    "block h-16 overflow-hidden rounded-lg ring-2 ring-transparent transition-shadow",
                    active
                      ? "ring-primary"
                      : "group-hover:ring-stage-foreground/25",
                  )}
                >
                  {renderThumb?.(item)}
                </span>
                <span className="mt-1.5 block truncate text-xs">
                  {baseName(item.path)}
                </span>
                <span
                  className={cn(
                    "text-2xs flex items-center gap-1 truncate tabular-nums",
                    s.tone === "error"
                      ? "text-destructive"
                      : s.tone === "done"
                        ? "text-success"
                        : "text-stage-muted",
                  )}
                  title={s.text}
                >
                  {s.tone === "done" && (
                    <IconCircleCheckFilled className="size-3 shrink-0" />
                  )}
                  {s.tone === "error" && (
                    <IconAlertCircleFilled className="size-3 shrink-0" />
                  )}
                  <span className="truncate">{s.text}</span>
                </span>
              </button>
              <button
                type="button"
                aria-label="목록에서 빼기"
                onClick={() => onRemove(item.path)}
                disabled={disabled}
                className="bg-stage/80 text-stage-foreground hover:bg-stage absolute top-1 right-1 hidden size-5 items-center justify-center rounded-full backdrop-blur-sm group-hover:flex focus-visible:flex disabled:hidden"
              >
                <IconX className="size-3" stroke={2} />
              </button>
            </li>
          )
        })}
        {trailing && <li className="shrink-0">{trailing}</li>}
      </ul>
    </div>
  )
}
