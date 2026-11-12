import { cn } from "@/shared/lib/utils"

import { isEnabled } from "../lib/is-enabled"
import type { Field } from "../lib/schema-to-fields"
import { ChipToggle } from "./chip-toggle"
import { OptionField } from "./option-field"

type Props = {
  fields: Field[]
  value: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  className?: string
}

/** 옵션 패널이 없는 화면(`x-layout`)에서 작업대 위에 한 줄로 늘어놓는 옵션. `x-short` 불리언은 칩으로 묶는다 */
export function OptionsBar({ fields, value, onChange, className }: Props) {
  const visible = fields.filter((f) => isEnabled(f, value))
  const chips = visible.filter(
    (f): f is Extract<Field, { kind: "checkbox" }> =>
      f.kind === "checkbox" && f.short !== undefined,
  )
  const rest = visible.filter((f) => !chips.includes(f as never))
  const set = (key: string, v: unknown) => onChange({ ...value, [key]: v })
  return (
    <div
      className={cn("flex flex-wrap items-center gap-x-6 gap-y-2", className)}
    >
      {chips.length > 0 && (
        <div className="flex items-center gap-1.5">
          {chips[0]?.group && (
            <span className="text-muted-foreground mr-1 text-sm">
              {chips[0].group}
            </span>
          )}
          {chips.map((f) => (
            <ChipToggle
              key={f.key}
              short={f.short ?? ""}
              label={f.description ? `${f.label} — ${f.description}` : f.label}
              checked={value[f.key] === true}
              onChange={(c) => set(f.key, c)}
            />
          ))}
        </div>
      )}
      {rest.map((f) => (
        <OptionField
          key={f.key}
          field={f}
          value={value[f.key]}
          onChange={(v) => set(f.key, v)}
          className="flex-none"
        />
      ))}
    </div>
  )
}
