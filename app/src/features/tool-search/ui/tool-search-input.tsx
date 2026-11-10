import { cn } from "@/shared/lib/utils"
import { Input } from "@/shared/ui/input"

type Props = {
  value: string
  onChange: (value: string) => void
  className?: string
}

export function ToolSearchInput({ value, onChange, className }: Props) {
  return (
    <Input
      type="search"
      aria-label="도구 검색"
      placeholder="도구 검색"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={cn("h-7", className)}
    />
  )
}
