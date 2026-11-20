import { useEffect, useRef } from "react"

import type { Block } from "@/entities/tool"
import { BlockList } from "@/features/interactive-result"
import { OptionsForm, type Field } from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { ErrorBanner } from "@/shared/ui/error-banner"

/** 작업대 가장자리에서 띄운 거리(px) = `inset-x-1.25 bottom-1.25` */
const OFFSET = 5

type Props = {
  fields: Field[]
  options: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  /** 패널 아래에 두는 결과 블록 (`x-panel-blocks`). `image`는 왼쪽 미리보기 칸 */
  blocks: Block[]
  /** 값이 null인 옵션 칸에 보여 줄 값 (`optionHints`) */
  hints?: Record<string, unknown>
  fetching: boolean
  error: string | null
  /** 있으면 하단 [닫기] (`x-panel-when` 옵션을 비운다) */
  onClose?: () => void
  /** 패널이 덮는 높이(px, 아래 띄운 거리 포함)가 바뀔 때. 닫히면 0 — 가려진 결과를 스크롤로 꺼낼 여백에 쓴다 */
  onHeight: (px: number) => void
  className?: string
}

/**
 * `x-panel-position: "bottom"`의 옵션 패널: 작업대 아래에서 올라와 결과 위에 떠 있는 카드.
 * 왼쪽 정사각형 칸에 `image` 블록(보기 전용 미리보기, 실제 크기로 그리되 칸보다 크면 줄임),
 * 오른쪽에 옵션 → 나머지 결과 블록 → 오류, 넘치면 안에서 스크롤.
 */
export function BottomPanel({
  fields,
  options,
  onChange,
  blocks,
  hints,
  fetching,
  error,
  onClose,
  onHeight,
  className,
}: Props) {
  const ref = useRef<HTMLElement>(null)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const ro = new ResizeObserver(() => onHeight(el.offsetHeight + OFFSET))
    ro.observe(el)
    return () => {
      ro.disconnect()
      onHeight(0)
    }
  }, [onHeight])

  const images = blocks.filter((b) => b.type === "image")
  const rest = blocks.filter((b) => b.type !== "image")
  const fading = fetching && "opacity-60"

  return (
    <section
      ref={ref}
      aria-label="옵션"
      className={cn(
        // 결과 뷰어의 작업대 색 토큰을 테마 색으로 (밝은 패널 위)
        "stage-plain bg-background text-foreground shadow-dialog animate-in slide-in-from-bottom absolute inset-x-1.25 bottom-1.25 z-10 flex max-h-[60%] flex-col rounded-xl duration-200",
        className,
      )}
    >
      <div className="flex min-h-0 flex-1 gap-5 overflow-y-auto px-5 pt-4 pb-3">
        {images.length > 0 && (
          <div
            className={cn(
              "bg-muted flex size-40 shrink-0 items-center justify-center overflow-hidden rounded-lg transition-opacity",
              fading,
            )}
          >
            {images.map((b, i) => (
              // 보기 전용: SVG의 width·height 그대로, 칸보다 크면 칸에 맞춰 줄인다
              <img
                key={i}
                src={b.src}
                alt=""
                className="max-h-full max-w-full object-contain"
              />
            ))}
          </div>
        )}
        <div className="flex min-w-0 flex-1 flex-col gap-4">
          {fields.length > 0 && (
            <OptionsForm
              fields={fields}
              value={options}
              hints={hints}
              onChange={onChange}
            />
          )}
          {rest.length > 0 && (
            <BlockList
              blocks={rest}
              className={cn("shrink-0 transition-opacity", fading)}
            />
          )}
          {error && (
            <ErrorBanner message={error} className="rounded-lg border" />
          )}
        </div>
      </div>
      {onClose && (
        <footer className="flex shrink-0 justify-end border-t px-5 py-3">
          <Button variant="outline" onClick={onClose}>
            닫기
          </Button>
        </footer>
      )}
    </section>
  )
}
