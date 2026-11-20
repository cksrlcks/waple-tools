import { useState } from "react"

import type { Block } from "@/entities/tool"
import { BlockList } from "@/features/interactive-result"
import { OptionsForm, type Field } from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

type Props = {
  fields: Field[]
  options: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  /** 패널 블록 (`x-panel-blocks`). `image`는 맨 위 미리보기 칸 */
  blocks: Block[]
  /** 값이 null인 옵션 칸에 보여 줄 값 (`optionHints`) */
  hints?: Record<string, unknown>
  /** 결과를 다시 만드는 중: 미리보기·버튼을 흐리게 */
  fading: boolean
  error: string | null
  className?: string
}

/**
 * 옵션 패널 내용 (아래 시트·별도 창 공용): 위에서부터 `image` 블록(보기 전용 미리보기, 실제 크기로 그리되
 * 칸보다 크면 줄이고 아래에 안내) → 옵션 → 나머지 결과 블록(버튼 묶음은 두 칸 격자) → 오류, 넘치면 안에서 스크롤.
 */
export function PanelContent({
  fields,
  options,
  onChange,
  blocks,
  hints,
  fading,
  error,
  className,
}: Props) {
  // 그림 원래 크기(SVG width·height)가 칸보다 커서 줄여 보이는가
  // ponytail: 마지막에 읽힌 그림 기준 (미리보기 그림은 보통 하나)
  const [overflow, setOverflow] = useState(false)

  const images = blocks.filter((b) => b.type === "image")
  const rest = blocks.filter((b) => b.type !== "image")
  const fade = fading && "opacity-60"

  return (
    <div
      className={cn(
        "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pb-3",
        className,
      )}
    >
      {images.length > 0 && (
        <div className="flex shrink-0 flex-col gap-1.5">
          <div
            className={cn(
              "bg-muted flex h-40 w-full items-center justify-center overflow-hidden rounded-lg transition-opacity",
              fade,
            )}
          >
            {images.map((b, i) => (
              // 보기 전용: SVG의 width·height 그대로, 칸보다 크면 칸에 맞춰 줄인다
              <img
                key={i}
                src={b.src}
                alt=""
                className="max-h-full max-w-full object-contain"
                onLoad={(e) => {
                  const img = e.currentTarget
                  const box = img.parentElement
                  if (!box) return
                  setOverflow(
                    img.naturalWidth > box.clientWidth ||
                      img.naturalHeight > box.clientHeight,
                  )
                }}
              />
            ))}
          </div>
          {overflow && (
            <p className="text-muted-foreground text-center text-xs">
              칸보다 커서 실제 크기로 보여 줄 수 없습니다.
            </p>
          )}
        </div>
      )}
      {fields.length > 0 && (
        <OptionsForm
          fields={fields}
          value={options}
          hints={hints}
          onChange={onChange}
          // 묶음 안 칸 사이를 넓히고(OptionsForm의 section은 gap-3) 마지막 칸(색) 아래 버튼 묶음까지 띄운다.
          // 보일 칸이 없으면 빈 칸을 남기지 않는다
          className="shrink-0 pb-3 empty:hidden [&>section]:gap-7"
        />
      )}
      {rest.length > 0 && (
        <BlockList
          blocks={rest}
          actionsLayout="grid"
          className={cn("shrink-0 transition-opacity", fade)}
        />
      )}
      {error && <ErrorBanner message={error} className="rounded-lg border" />}
    </div>
  )
}
