import { cn } from "@/shared/lib/utils"

import { isEnabled } from "../lib/is-enabled"
import type { Field } from "../lib/schema-to-fields"
import { toSections } from "../lib/to-sections"
import { OptionField } from "./option-field"
import { SectionHeader } from "./section-header"

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
  const set = (key: string, v: unknown) => onChange({ ...value, [key]: v })
  return (
    <div className={cn("flex flex-col gap-5", className)}>
      {toSections(fields).map((section, i) => (
        <section key={section.title ?? i} className="flex flex-col gap-3">
          {section.title && (
            <SectionHeader
              title={section.title}
              toggle={section.toggle}
              checked={
                section.toggle ? value[section.toggle.key] === true : false
              }
              onToggle={(c) => section.toggle && set(section.toggle.key, c)}
              disabled={disabled}
            />
          )}
          {section.fields.map((f) => {
            // 다른 옵션 값 때문에 쓰지 않는 칸은 흐리게 비활성
            const off = !isEnabled(f, value)
            return (
              <OptionField
                key={f.key}
                field={f}
                value={value[f.key]}
                onChange={(v) => set(f.key, v)}
                disabled={disabled || off}
                className={cn(section.toggle && "pl-6", off && "opacity-50")}
              />
            )
          })}
        </section>
      ))}
    </div>
  )
}
