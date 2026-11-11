import type { ReactNode } from "react"
import {
  IconAlertCircleFilled,
  IconCircleCheckFilled,
  IconX,
} from "@tabler/icons-react"

import type { FileResult, PathStat } from "@/entities/tool"
import { baseName } from "@/shared/lib/base-name"
import { cn } from "@/shared/lib/utils"

import { statusOf } from "../lib/status-of"

type Props = {
  item: PathStat
  result?: FileResult
  thumb?: ReactNode
  /** 상태 앞에 둘 것 (파일마다 고르는 옵션) */
  extra?: ReactNode
  onRemove: () => void
  disabled?: boolean
  className?: string
}

/** 파일 목록의 한 줄: 아이콘 · 이름 · 형식 · 크기(실행 뒤에는 결과 파일) · 빼기 */
export function FileRow({
  item,
  result,
  thumb,
  extra,
  onRemove,
  disabled,
  className,
}: Props) {
  const s = statusOf(item, result)
  const ext = item.isDir ? null : /\.([^.\\/]+)$/.exec(item.path)?.[1]
  // 끝났으면 무엇이 만들어졌는지 보여 준다 (예: → logo.woff2)
  const output =
    result?.status === "done" && result.outputs[0]
      ? `→ ${baseName(result.outputs[0])}`
      : null
  return (
    <li
      className={cn(
        "group hover:bg-stage-foreground/5 flex h-11 items-center gap-3 rounded-md px-2",
        className,
      )}
    >
      <span className="size-8 shrink-0 overflow-hidden rounded">{thumb}</span>
      <span className="min-w-0 flex-1 truncate text-sm" title={item.path}>
        {baseName(item.path)}
      </span>
      {ext && (
        <span className="bg-stage-foreground/8 text-stage-muted text-2xs shrink-0 rounded px-1.5 py-0.5 font-medium uppercase">
          {ext}
        </span>
      )}
      {extra}
      <span
        className={cn(
          "flex max-w-60 shrink-0 items-center gap-1 truncate text-xs tabular-nums",
          s.tone === "error"
            ? "text-destructive"
            : s.tone === "done"
              ? "text-success"
              : "text-stage-muted",
        )}
        title={output ?? s.text}
      >
        {s.tone === "done" && (
          <IconCircleCheckFilled className="size-3.5 shrink-0" />
        )}
        {s.tone === "error" && (
          <IconAlertCircleFilled className="size-3.5 shrink-0" />
        )}
        <span className="truncate">{output ?? s.text}</span>
      </span>
      <button
        type="button"
        aria-label="목록에서 빼기"
        onClick={onRemove}
        disabled={disabled}
        className="text-stage-muted hover:text-stage-foreground hover:bg-stage-foreground/10 invisible flex size-6 shrink-0 items-center justify-center rounded group-hover:visible focus-visible:visible disabled:hidden"
      >
        <IconX className="size-3.5" stroke={2} />
      </button>
    </li>
  )
}
