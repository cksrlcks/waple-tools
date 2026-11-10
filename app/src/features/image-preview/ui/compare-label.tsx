import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{ className?: string }>

/** 작업대 위 이미지 모서리의 "원본"·"결과" 라벨 */
export function CompareLabel({ children, className }: Props) {
  return (
    <span
      className={cn(
        "text-2xs bg-stage-foreground/15 text-stage-foreground pointer-events-none absolute rounded-md px-2 py-1 font-medium backdrop-blur-sm",
        className,
      )}
    >
      {children}
    </span>
  )
}
