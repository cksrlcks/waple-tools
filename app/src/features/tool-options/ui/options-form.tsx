import { cn } from "@/shared/lib/utils"

import type { Field } from "../lib/schema-to-fields"
import { OptionField } from "./option-field"

type Props = {
  fields: Field[]
  value: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  disabled?: boolean
  className?: string
}

export function OptionsForm({
  fields,
  value,
  onChange,
  disabled,
  className,
}: Props) {
  if (fields.length === 0) {
    return (
      <p className={cn("text-muted-foreground text-sm", className)}>
        설정할 옵션이 없습니다.
      </p>
    )
  }
  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {fields.map((f) => (
        <OptionField
          key={f.key}
          field={f}
          value={value[f.key]}
          onChange={(v) => onChange({ ...value, [f.key]: v })}
          disabled={disabled}
        />
      ))}
    </div>
  )
}
