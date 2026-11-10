import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{
  tone?: "error"
  className?: string
}>

/** 미리보기 자리를 채우는 안내·오류 문구 */
export function PreviewNotice({ children, tone, className }: Props) {
  return (
    <div
      className={cn(
        "bg-muted/60 flex aspect-[4/3] items-center justify-center rounded-md px-6 text-center text-sm",
        tone === "error" ? "text-destructive" : "text-muted-foreground",
        className,
      )}
    >
      {children}
    </div>
  )
}
