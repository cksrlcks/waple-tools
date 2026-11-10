import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{
  tone?: "error"
  className?: string
}>

/** 작업대 가운데 안내·오류 문구 */
export function PreviewNotice({ children, tone, className }: Props) {
  return (
    <div
      className={cn(
        "flex size-full items-center justify-center px-8 text-center text-sm",
        tone === "error" ? "text-destructive" : "text-stage-muted",
        className,
      )}
    >
      {children}
    </div>
  )
}
