import { useState } from "react"

import { cn } from "@/shared/lib/utils"
import { Input } from "@/shared/ui/input"

import { parseSliderInput } from "../lib/parse-slider-input"

type Props = {
  value: unknown
  field: { min: number; max: number; step: number; nullable?: boolean }
  /** 값이 null일 때 흐리게 보여 줄 값 (원래 값) */
  hint?: unknown
  labelId: string
  onChange: (value: number | null) => void
  disabled?: boolean
  className?: string
}

/**
 * 슬라이더 옆 숫자 입력칸 (`x-slider-input`). 치는 동안은 글만 바꾸고, 칸을 벗어나거나 Enter에서
 * 범위·단위에 맞춘 값으로 반영한다. 비우면 null 허용 칸은 원래대로, 읽을 수 없으면 되돌린다.
 */
export function SliderInput({
  value,
  field,
  hint,
  labelId,
  onChange,
  disabled,
  className,
}: Props) {
  const shown = value === null || value === undefined ? "" : String(value)
  const [text, setText] = useState(shown)
  // 슬라이더·[원래대로]로 값이 바뀌면 칸 글도 따라간다
  const [prev, setPrev] = useState(shown)
  if (prev !== shown) {
    setPrev(shown)
    setText(shown)
  }

  const commit = () => {
    const v = parseSliderInput(text, field)
    if (v === undefined) return setText(shown)
    setText(v === null ? "" : String(v))
    if (v !== (value ?? null)) onChange(v)
  }

  return (
    <Input
      type="text"
      inputMode="decimal"
      aria-labelledby={labelId}
      placeholder={typeof hint === "number" ? String(hint) : undefined}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit()
      }}
      disabled={disabled}
      className={cn(
        "h-7 w-14 shrink-0 px-1.5 text-right text-xs tabular-nums",
        className,
      )}
    />
  )
}
