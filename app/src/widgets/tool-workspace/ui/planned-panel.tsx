import { IconHourglassHigh } from "@tabler/icons-react"

import type { ToolEntry } from "@/entities/tool"

import { ToolPlaceholder } from "./tool-placeholder"

type Props = {
  tool: ToolEntry
  className?: string
}

/** 아직 만들지 않은 도구 (카탈로그 `planned`) */
export function PlannedPanel({ tool, className }: Props) {
  return (
    <ToolPlaceholder tool={tool} className={className}>
      <p className="bg-muted text-muted-foreground flex items-center gap-1.5 rounded-md px-2.5 py-1 text-sm">
        <IconHourglassHigh className="size-4" stroke={1.75} />
        준비 중인 도구입니다
      </p>
    </ToolPlaceholder>
  )
}
