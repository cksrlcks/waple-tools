import type { ToolEntry } from "@/entities/tool"
import { Button } from "@/shared/ui/button"

import { ToolPlaceholder } from "./tool-placeholder"

type Props = {
  tool: ToolEntry
  className?: string
}

export function NotInstalledPanel({ tool, className }: Props) {
  return (
    <ToolPlaceholder tool={tool} className={className}>
      <Button size="lg" disabled>
        설치
      </Button>
      <p className="text-muted-foreground text-xs">
        추가 설치는 다음 업데이트에서 지원합니다.
      </p>
    </ToolPlaceholder>
  )
}
