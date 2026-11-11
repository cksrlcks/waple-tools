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
  /** 다른 곳(미리보기 영역 편집 등)에서 정하는 옵션: 폼에서 숨긴다 */
  hidden?: string[]
  /** 옵션은 아니지만 보일지 판단에 쓰는 값 (예: `$inputExt` 넣은 파일 확장자) */
  context?: Record<string, unknown>
  className?: string
}

export function OptionsForm({
  fields,
  value,
  onChange,
  disabled,
  hidden = [],
  context,
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
    <div className={cn("divide-border flex flex-col divide-y", className)}>
      {toSections(fields).map((section, i) => {
        const on = section.toggle ? value[section.toggle.key] === true : true
        // 다른 옵션 값 때문에 쓰지 않는 칸은 숨긴다 (값은 그대로 남는다)
        const visible = section.fields.filter(
          (f) =>
            isEnabled(f, { ...value, ...context }) && !hidden.includes(f.key),
        )
        // 보일 칸이 하나도 없는 묶음은 제목까지 숨긴다 (예: 웹 파비콘을 끄면 "파비콘 파일")
        if (!section.toggle && visible.length === 0) return null
        return (
          <section
            key={section.title ?? i}
            className="flex flex-col gap-3 py-4 first:pt-0 last:pb-0"
          >
            {section.title && (
              <SectionHeader
                title={section.title}
                toggle={section.toggle}
                checked={on}
                onToggle={(c) => section.toggle && set(section.toggle.key, c)}
                disabled={disabled}
              />
            )}
            {section.note && (
              <p className="text-muted-foreground -mt-1.5 text-xs">
                {section.note}
              </p>
            )}
            {/* 스위치가 꺼진 묶음은 칸을 접는다 (값은 그대로 남는다) */}
            {on &&
              visible.map((f) => (
                <OptionField
                  key={f.key}
                  field={f}
                  value={value[f.key]}
                  onChange={(v) => set(f.key, v)}
                  disabled={disabled}
                />
              ))}
          </section>
        )
      })}
    </div>
  )
}
