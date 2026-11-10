import { useState } from "react"
import {
  IconArrowsHorizontal,
  IconLayoutColumns,
  IconLoader2,
} from "@tabler/icons-react"

import { baseName } from "@/shared/lib/base-name"
import { formatChange } from "@/shared/lib/format-bytes"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"

import { useToolPreview } from "../api/use-tool-preview"
import type { PreviewMode } from "../lib/preview-mode"
import { CompareSideBySide } from "./compare-side-by-side"
import { CompareSlider } from "./compare-slider"
import { PreviewNotice as Notice } from "./preview-notice"
import { ResultView } from "./result-view"

type Props = {
  toolId: string
  /** 미리 볼 파일 (없으면 안내 문구) */
  path: string | null
  opts: Record<string, unknown>
  /** 실행할 수 없는 옵션이면 미리보기도 만들지 않는다 */
  blockedReason?: string | null
  /** compare: 원본과 비교, result: 결과만 (모양이 바뀌는 도구) */
  mode?: PreviewMode
  className?: string
}

type View = "slider" | "side"

/** 고른 파일 하나를 지금 옵션으로 임시 처리해 원본과 비교한다 (실제 파일은 바뀌지 않음) */
export function PreviewPane({
  toolId,
  path,
  opts,
  blockedReason,
  mode = "compare",
  className,
}: Props) {
  const [view, setView] = useState<View>("slider")
  // 결과만 보는 도구는 막혀 있어도(아무것도 안 고름) 원본을 보여 주려고 요청한다
  const blocked = mode === "compare" ? blockedReason : null
  const preview = useToolPreview(toolId, blocked ? null : path, opts)
  const data = preview.data

  let body
  if (!path) {
    body = <Notice>파일을 추가하면 결과를 미리 볼 수 있습니다.</Notice>
  } else if (blocked) {
    body = <Notice>{blocked}</Notice>
  } else if (preview.isError && !data) {
    body = <Notice tone="error">{toUserMessage(preview.error)}</Notice>
  } else if (!data) {
    body = <Notice>미리보기를 만드는 중…</Notice>
  } else if (!data.after && mode === "result") {
    // 아직 바꿀 것을 고르지 않았거나 실패: 원본을 그대로 보여 주고 이유는 아래에
    body = <ResultView shown={data.before} label="원본" />
  } else if (!data.after) {
    body = (
      <Notice tone={data.error ? "error" : undefined}>
        {data.error ?? "이 도구의 결과는 미리 볼 수 없습니다."}
      </Notice>
    )
  } else if (mode === "result") {
    body = <ResultView shown={data.after} />
  } else {
    body =
      view === "slider" ? (
        <CompareSlider before={data.before} after={data.after} />
      ) : (
        <CompareSideBySide before={data.before} after={data.after} />
      )
  }

  const views = [
    { v: "slider", label: "슬라이더로 비교", Icon: IconArrowsHorizontal },
    { v: "side", label: "나란히 비교", Icon: IconLayoutColumns },
  ] as const
  const note =
    mode === "result" && data && !data.after && data.error
      ? { text: data.error, error: false }
      : preview.isError && data
        ? { text: toUserMessage(preview.error), error: true }
        : null

  return (
    <section className={cn("flex min-h-0 flex-col", className)}>
      <div className="flex h-11 shrink-0 items-center gap-3 px-4">
        <span
          className="min-w-0 truncate text-sm font-medium"
          title={path ?? ""}
        >
          {path ? baseName(path) : "미리보기"}
        </span>
        {data && (
          <span className="text-stage-muted shrink-0 text-xs tabular-nums">
            {data.before.width}×{data.before.height}
          </span>
        )}
        {preview.isFetching && data && (
          <IconLoader2
            aria-label="다시 만드는 중"
            className="text-stage-muted size-3.5 shrink-0 animate-spin"
            stroke={2}
          />
        )}
        <div className="ml-auto flex shrink-0 items-center gap-2">
          {data?.after && data.sizeAfter !== null && (
            <>
              <span className="bg-stage-foreground/8 rounded-md px-2 py-1 text-xs tabular-nums">
                {formatChange(data.sizeBefore, data.sizeAfter)}
              </span>
              {(data.before.width !== data.after.width ||
                data.before.height !== data.after.height) && (
                <span className="bg-stage-foreground/8 rounded-md px-2 py-1 text-xs tabular-nums">
                  → {data.after.width}×{data.after.height}
                </span>
              )}
            </>
          )}
          {mode === "compare" && (
            <div className="bg-stage-foreground/8 flex rounded-md p-0.5">
              {views.map(({ v, label, Icon }) => (
                <button
                  key={v}
                  type="button"
                  aria-label={label}
                  aria-pressed={view === v}
                  title={label}
                  onClick={() => setView(v)}
                  className={cn(
                    "focus-visible:ring-primary flex size-6 items-center justify-center rounded outline-none focus-visible:ring-2",
                    view === v
                      ? "bg-stage-foreground/16 text-stage-foreground"
                      : "text-stage-muted hover:text-stage-foreground",
                  )}
                >
                  <Icon className="size-3.5" stroke={1.75} />
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="relative min-h-0 flex-1 px-4 pb-4">{body}</div>
      {note && (
        <p
          className={cn(
            "-mt-2 px-4 pb-3 text-xs",
            note.error ? "text-destructive" : "text-stage-muted",
          )}
        >
          {note.text}
        </p>
      )}
    </section>
  )
}
