import { cn } from "@/shared/lib/utils"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select"

import type { Option } from "../config/options"

type Props = {
  options: Option[]
  value: string
  onChange: (value: string) => void
  className?: string
}

export function ChoiceSelect({ options, value, onChange, className }: Props) {
  return (
    <Select
      items={options}
      value={value}
      onValueChange={(v) => onChange(String(v))}
    >
      <SelectTrigger className={cn("min-w-56", className)}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={o.value} value={o.value}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
