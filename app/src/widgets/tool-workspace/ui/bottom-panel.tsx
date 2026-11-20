import { useEffect, useRef } from "react"

import type { Block } from "@/entities/tool"
import { BlockList } from "@/features/interactive-result"
import { OptionsForm, type Field } from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { ErrorBanner } from "@/shared/ui/error-banner"

type Props = {
  fields: Field[]
  options: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  /** 패널 아래에 두는 결과 블록 (`x-panel-blocks`) */
  blocks: Block[]
  fetching: boolean
  error: string | null
  /** 있으면 하단 [닫기] (`x-panel-when` 옵션을 비운다) */
  onClose?: () => void
  /** 패널 높이(px)가 바뀔 때. 닫히면 0 — 가려진 결과를 스크롤로 꺼낼 여백에 쓴다 */
  onHeight: (px: number) => void
  className?: string
}

/**
 * `x-panel-position: "bottom"`의 옵션 패널: 작업대 아래에서 올라와 결과 위를 덮는다.
 * 도구 이름 없이 옵션 → 결과 블록 → 오류, 넘치면 안에서 스크롤.
 */
export function BottomPanel({
  fields,
  options,
  onChange,
  blocks,
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
    const ro = new ResizeObserver(() => onHeight(el.offsetHeight))
    ro.observe(el)
    return () => {
      ro.disconnect()
      onHeight(0)
    }
  }, [onHeight])

  return (
    <section
      ref={ref}
      aria-label="옵션"
      className={cn(
        // 결과 뷰어의 작업대 색 토큰을 테마 색으로 (밝은 패널 위)
        "stage-plain bg-background text-foreground shadow-dialog animate-in slide-in-from-bottom absolute inset-x-0 bottom-0 z-10 flex max-h-[60%] flex-col rounded-t-xl duration-200",
        className,
      )}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 pt-4 pb-3">
        {fields.length > 0 && (
          <OptionsForm fields={fields} value={options} onChange={onChange} />
        )}
        {blocks.length > 0 && (
          <BlockList
            blocks={blocks}
            className={cn(
              "shrink-0 transition-opacity",
              fetching && "opacity-60",
            )}
          />
        )}
        {error && <ErrorBanner message={error} className="rounded-lg border" />}
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
