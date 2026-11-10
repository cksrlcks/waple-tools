import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{
  label: string
  className?: string
}>

export function SettingRow({ label, children, className }: Props) {
  return (
    <div
      className={cn(
        "flex min-h-14 items-center justify-between gap-4 border-b py-3",
        className,
      )}
    >
      <span className="text-sm">{label}</span>
      {children}
    </div>
  )
}
