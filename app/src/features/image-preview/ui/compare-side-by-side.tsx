import type { Shown } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { CompareLabel } from "./compare-label"

type Props = {
  before: Shown
  after: Shown
  className?: string
}

/** 원본과 결과를 나란히 */
export function CompareSideBySide({ before, after, className }: Props) {
  return (
    <div className={cn("grid size-full grid-cols-2 gap-3", className)}>
      {[
        { label: "원본", shown: before },
        { label: "결과", shown: after },
      ].map(({ label, shown }) => (
        <div key={label} className="relative min-h-0">
          <img
            src={shown.url}
            alt={label}
            className="absolute inset-0 size-full object-contain"
          />
          <CompareLabel className="top-0 left-0">{label}</CompareLabel>
        </div>
      ))}
    </div>
  )
}
