import type { ReactNode } from "react"

import { toolIcon, type ToolEntry } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  tool: ToolEntry
  /** 설명 아래 (버튼·안내 문구) */
  children?: ReactNode
  className?: string
}

/** 작업 화면을 아직 쓸 수 없는 도구의 가운데 안내 (설치 안 됨·준비 중) */
export function ToolPlaceholder({ tool, children, className }: Props) {
  const Icon = toolIcon(tool.id)
  return (
    <section
      className={cn(
        "m-auto flex max-w-sm flex-col items-center gap-4 px-6 text-center",
        className,
      )}
    >
      <span className="bg-primary/10 text-primary flex size-14 items-center justify-center rounded-2xl">
        <Icon className="size-7" stroke={1.5} />
      </span>
      <div className="flex flex-col gap-1">
        <h1 className="text-base font-semibold">{tool.name}</h1>
        <p className="text-muted-foreground">{tool.description}</p>
      </div>
      {children}
    </section>
  )
}
