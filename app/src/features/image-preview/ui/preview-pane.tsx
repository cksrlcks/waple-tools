import { useState } from "react"

import { formatChange } from "@/shared/lib/format-bytes"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

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
  const preview = useToolPreview(toolId, blockedReason ? null : path, opts)
  const data = preview.data

  let body
  if (!path) {
    body = <Notice>파일을 추가하면 결과를 미리 볼 수 있습니다.</Notice>
  } else if (blockedReason) {
    body = <Notice>{blockedReason}</Notice>
  } else if (preview.isError && !data) {
    body = <Notice tone="error">{toUserMessage(preview.error)}</Notice>
  } else if (!data) {
    body = <Notice>미리보기를 만드는 중…</Notice>
  } else if (!data.after) {
    body = <Notice>이 도구의 결과는 미리 볼 수 없습니다.</Notice>
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

  return (
    <section className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-center justify-between">
        <h2 className="text-2xs text-muted-foreground font-medium">
          미리보기 {preview.isFetching && data ? "· 다시 만드는 중…" : ""}
        </h2>
        <div className={cn("flex gap-1", mode === "result" && "invisible")}>
          {(
            [
              ["slider", "슬라이더"],
              ["side", "나란히"],
            ] as const
          ).map(([v, label]) => (
            <Button
              key={v}
              size="sm"
              variant={view === v ? "secondary" : "ghost"}
              onClick={() => setView(v)}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>
      {body}
      {data?.after && data.sizeAfter !== null && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
          <dt className="text-muted-foreground">용량</dt>
          <dd className="tabular-nums">
            {formatChange(data.sizeBefore, data.sizeAfter)}
          </dd>
          <dt className="text-muted-foreground">크기</dt>
          <dd className="tabular-nums">
            {data.before.width}×{data.before.height} → {data.after.width}×
            {data.after.height}
          </dd>
        </dl>
      )}
      {preview.isError && data && (
        <p className="text-destructive text-xs">
          {toUserMessage(preview.error)}
        </p>
      )}
    </section>
  )
}
