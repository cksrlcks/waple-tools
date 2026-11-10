import type { Shown } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { CompareLabel } from "./compare-label"

type Props = {
  shown: Shown
  /** 모서리 라벨 (기본 "결과") */
  label?: string
  className?: string
}

/** 결과 이미지 하나 (모양·크기가 바뀌는 도구용) */
export function ResultView({ shown, label = "결과", className }: Props) {
  return (
    <div className={cn("relative size-full", className)}>
      <img
        src={shown.url}
        alt={label}
        className="absolute inset-0 size-full object-contain"
      />
      <CompareLabel className="top-0 left-0">{label}</CompareLabel>
    </div>
  )
}
