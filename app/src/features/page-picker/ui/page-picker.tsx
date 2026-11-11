import { useRef } from "react"
import { IconLoader2 } from "@tabler/icons-react"

import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"

import { usePageThumbs } from "../api/use-page-thumbs"
import { formatPages, parsePages } from "../lib/page-set"
import { PageThumb } from "./page-thumb"

type Props = {
  toolId: string
  /** 썸네일을 만들 파일 (없으면 안내 문구) */
  path: string | null
  /** 페이지 범위 옵션 값 (빈 칸 = 전체) */
  value: string
  onChange: (value: string) => void
  /** false면 쪽을 보여 주기만 한다 (대상 옵션이 비활성일 때, 예: 분할의 "페이지마다") */
  selectable?: boolean
  disabled?: boolean
  className?: string
}

/** 작업대의 페이지 고르기: 쪽마다 썸네일, 누르면 고르기/빼기, Shift로 범위 (`x-page-picker`) */
export function PagePicker({
  toolId,
  path,
  value,
  onChange,
  selectable = true,
  disabled,
  className,
}: Props) {
  const thumbs = usePageThumbs(toolId, path)
  // Shift 범위 선택의 시작점
  const last = useRef<number | null>(null)

  const notice = (text: string, error = false) => (
    <div
      className={cn(
        "flex items-center justify-center gap-2 px-8 text-center text-sm",
        error ? "text-destructive" : "text-stage-muted",
        className,
      )}
    >
      {!error && <IconLoader2 className="size-4 animate-spin" stroke={2} />}
      {text}
    </div>
  )
  if (!path) return notice("PDF를 넣으면 페이지를 볼 수 있습니다.")
  if (thumbs.isError) return notice(toUserMessage(thumbs.error), true)
  if (!thumbs.data) return notice("페이지를 읽는 중…")

  const urls = thumbs.data
  const total = urls.length
  const selected = parsePages(value, total)

  const toggle = (page: number, shift: boolean) => {
    const next = new Set(selected)
    const on = !selected.has(page)
    const from = shift && last.current !== null ? last.current : page
    for (let p = Math.min(from, page); p <= Math.max(from, page); p++) {
      if (on) next.add(p)
      else next.delete(p)
    }
    last.current = page
    onChange(formatPages(next, total))
  }

  if (!selectable) {
    return (
      <section className={cn("flex min-h-0 flex-col", className)}>
        <div className="flex h-11 shrink-0 items-center px-4">
          <span className="text-stage-foreground text-sm font-medium tabular-nums">
            {total}쪽
          </span>
        </div>
        <ul className="grid min-h-0 flex-1 auto-rows-min grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-4 overflow-y-auto px-4 pt-1 pb-4">
          {urls.map((url, i) => (
            <li key={i} className="flex justify-center">
              <PageThumb
                url={url}
                page={i + 1}
                selected
                selectable={false}
                onToggle={() => {}}
              />
            </li>
          ))}
        </ul>
      </section>
    )
  }

  const action =
    "hover:text-stage-foreground disabled:pointer-events-none disabled:opacity-50"
  return (
    <section className={cn("flex min-h-0 flex-col", className)}>
      <div className="text-stage-muted flex h-11 shrink-0 items-center gap-3 px-4 text-xs">
        <span className="text-stage-foreground text-sm font-medium tabular-nums">
          {total}쪽 중 {selected.size}쪽 선택
        </span>
        <span className="hidden sm:inline">
          Shift를 누른 채 누르면 범위로 고릅니다
        </span>
        <div className="ml-auto flex items-center gap-3">
          <button
            type="button"
            className={action}
            disabled={disabled || selected.size === total}
            onClick={() =>
              onChange(formatPages(new Set(urls.map((_, i) => i + 1)), total))
            }
          >
            전체 선택
          </button>
          <button
            type="button"
            className={action}
            disabled={disabled || selected.size === 0}
            onClick={() => onChange(formatPages(new Set(), total))}
          >
            선택 해제
          </button>
        </div>
      </div>
      <ul className="grid min-h-0 flex-1 auto-rows-min grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-4 overflow-y-auto px-4 pt-1 pb-4">
        {urls.map((url, i) => (
          <li key={i} className="flex justify-center">
            <PageThumb
              url={url}
              page={i + 1}
              selected={selected.has(i + 1)}
              onToggle={(shift) => toggle(i + 1, shift)}
              disabled={disabled}
            />
          </li>
        ))}
      </ul>
    </section>
  )
}
