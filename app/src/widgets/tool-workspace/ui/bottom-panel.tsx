import { useEffect, useRef } from "react"
import { IconX } from "@tabler/icons-react"

import type { Block } from "@/entities/tool"
import type { Field } from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { PanelContent } from "./panel-content"

/** 작업대 가장자리에서 띄운 거리(px) = `right-1.25 bottom-1.25` */
const OFFSET = 5

type Props = {
  fields: Field[]
  options: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  /** 패널 아래에 두는 결과 블록 (`x-panel-blocks`). `image`는 맨 위 미리보기 칸 */
  blocks: Block[]
  /** 값이 null인 옵션 칸에 보여 줄 값 (`optionHints`) */
  hints?: Record<string, unknown>
  fetching: boolean
  error: string | null
  /** 있으면 오른쪽 위 닫기(X) 버튼 (`x-panel-when` 옵션을 비운다) */
  onClose?: () => void
  /** 패널이 덮는 높이(px, 아래 띄운 거리 포함)가 바뀔 때. 닫히면 0 — 가려진 결과를 스크롤로 꺼낼 여백에 쓴다 */
  onHeight: (px: number) => void
  className?: string
}

/**
 * `x-panel-position: "bottom"`의 옵션 패널: 작업대 오른쪽 아래에서 올라와 결과 위에 떠 있는 좁은 카드.
 * 내용은 `PanelContent`(별도 창과 같음). 닫기는 오른쪽 위 X 하나(높이를 아낀다).
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
  return (
    <section
      ref={ref}
      aria-label="옵션"
      className={cn(
        // 결과 뷰어의 작업대 색 토큰을 테마 색으로 (밝은 패널 위)
        "stage-plain bg-background text-foreground shadow-dialog animate-in slide-in-from-bottom absolute right-1.25 bottom-1.25 z-10 flex max-h-[calc(100%-0.625rem)] w-80 flex-col rounded-xl duration-200",
        className,
      )}
    >
      {onClose && (
        // 닫기 버튼만 두는 얇은 머리 줄: 아래 미리보기와 겹치지 않게
        <div className="flex shrink-0 justify-end px-2 pt-2">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="닫기"
            className="text-muted-foreground"
            onClick={onClose}
          >
            <IconX stroke={1.75} />
          </Button>
        </div>
      )}
      <PanelContent
        fields={fields}
        options={options}
        onChange={onChange}
        blocks={blocks}
        hints={hints}
        fading={fetching}
        error={error}
        className={onClose ? "pt-1" : "pt-4"}
      />
    </section>
  )
}
