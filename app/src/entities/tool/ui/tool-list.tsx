import { cn } from "@/shared/lib/utils"

import type { ToolInfo } from "../model/types"

type Props = {
  tools: ToolInfo[]
  selectedId: string | null
  onSelect: (id: string) => void
  className?: string
}

export function ToolList({ tools, selectedId, onSelect, className }: Props) {
  return (
    <ul className={cn("flex flex-col", className)}>
      {tools.map(({ meta }) => (
        <li key={meta.id}>
          <button
            type="button"
            onClick={() => onSelect(meta.id)}
            className={cn(
              "hover:bg-sidebar-accent my-px flex h-8 w-full items-center gap-2 rounded-md px-2 text-left text-sm font-medium",
              meta.id === selectedId && "bg-selection hover:bg-selection",
            )}
          >
            {meta.name}
          </button>
        </li>
      ))}
    </ul>
  )
}
