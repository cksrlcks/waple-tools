import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{
  label: string
  /** 이름 아래 작은 설명 */
  description?: string
  className?: string
}>

export function SettingRow({ label, description, children, className }: Props) {
  return (
    <div
      className={cn(
        "flex min-h-14 items-center justify-between gap-6 py-3",
        className,
      )}
    >
      <div className="flex min-w-0 flex-col gap-1">
        <span className="text-sm font-semibold">{label}</span>
        {description && (
          <span className="text-muted-foreground text-xs">{description}</span>
        )}
      </div>
      {children}
    </div>
  )
}
