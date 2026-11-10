import { IconSearch } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Input } from "@/shared/ui/input"

type Props = {
  value: string
  onChange: (value: string) => void
  className?: string
}

export function ToolSearchInput({ value, onChange, className }: Props) {
  return (
    <div className={cn("relative", className)}>
      <IconSearch
        className="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
        stroke={1.75}
      />
      <Input
        type="search"
        aria-label="도구 검색"
        placeholder="도구 검색"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="bg-muted h-9 border-transparent pl-8 shadow-none"
      />
    </div>
  )
}
