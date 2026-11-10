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
    </ToolPlaceholder>
  )
}
