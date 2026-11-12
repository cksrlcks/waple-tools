import type { PropsWithChildren } from "react"

import { cn } from "@/shared/lib/utils"

type Props = PropsWithChildren<{
  title: string
  className?: string
}>

/** 설정 묶음: 작은 제목 아래 항목들, 항목 사이는 가는 선 */
export function SettingGroup({ title, children, className }: Props) {
  return (
    <section className={cn("flex flex-col", className)}>
      <h3 className="text-muted-foreground pb-1 text-xs font-semibold">
        {title}
      </h3>
      <div className="divide-border flex flex-col divide-y">{children}</div>
    </section>
  )
}
