import type { ToolEntry } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

type Props = {
  tool: ToolEntry
  className?: string
}

export function NotInstalledPanel({ tool, className }: Props) {
  return (
    <section className={cn("flex flex-col gap-3", className)}>
      <header>
        <h1 className="text-base font-semibold">{tool.name}</h1>
        <p className="text-muted-foreground">{tool.description}</p>
      </header>
      <div className="flex items-center gap-3">
        <Button disabled>설치</Button>
        <span className="text-muted-foreground text-xs">
          추가 설치는 다음 업데이트에서 지원합니다.
        </span>
      </div>
    </section>
  )
}
