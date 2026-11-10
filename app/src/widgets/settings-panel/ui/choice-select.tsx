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
  /** long: 긴 문구(테마·저장 방식), short: 짧은 문구(지우기·30일) */
  width?: "long" | "short"
  className?: string
}

export function ChoiceSelect({
  options,
  value,
  onChange,
  width = "long",
  className,
}: Props) {
  return (
    <Select
      items={options}
      value={value}
      onValueChange={(v) => onChange(String(v))}
    >
      <SelectTrigger
        className={cn("px-2.5", width === "long" ? "w-72" : "w-32", className)}
      >
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
