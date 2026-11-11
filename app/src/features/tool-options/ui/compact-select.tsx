import { cn } from "@/shared/lib/utils"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select"

import type { Field } from "../lib/schema-to-fields"

type Props = {
  field: Extract<Field, { kind: "select" }>
  value: string
  onChange: (value: string) => void
  disabled?: boolean
  className?: string
}

/** 라벨 없는 작은 선택 칸 (작업대의 파일 목록 줄 안에서 파일마다 고를 때). 어두운 작업대 색에 맞춘다 */
export function CompactSelect({
  field,
  value,
  onChange,
  disabled,
  className,
}: Props) {
  return (
    <Select
      items={field.options}
      value={value}
      onValueChange={(v) => onChange(String(v))}
      disabled={disabled}
    >
      <SelectTrigger
        size="sm"
        aria-label={field.label}
        className={cn(
          "bg-stage-foreground/8 border-stage-foreground/15 text-stage-foreground dark:bg-stage-foreground/8",
          className,
        )}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {field.options.map((o) => (
          <SelectItem key={o.value} value={o.value}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
