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
    <div className={cn("grid grid-cols-2 gap-2", className)}>
      {[
        { label: "원본", shown: before },
        { label: "결과", shown: after },
      ].map(({ label, shown }) => (
        <div
          key={label}
          className="bg-muted relative aspect-[4/3] overflow-hidden rounded-md"
        >
          <img
            src={shown.url}
            alt={label}
            className="absolute inset-0 size-full object-contain"
          />
          <CompareLabel className="top-2 left-2">{label}</CompareLabel>
        </div>
      ))}
    </div>
  )
}
