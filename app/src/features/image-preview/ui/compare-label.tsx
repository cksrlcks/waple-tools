import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{ className?: string }>

/** 비교 화면 모서리의 "원본"·"결과" 라벨 */
export function CompareLabel({ children, className }: Props) {
  return (
    <span
      className={cn(
        "text-2xs bg-background/80 pointer-events-none absolute rounded px-1.5 py-0.5",
        className,
      )}
    >
      {children}
    </span>
  )
}
