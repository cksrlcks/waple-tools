import { cn } from "@/shared/lib/utils"
import { Checkbox } from "@/shared/ui/checkbox"

import type { Field } from "../lib/schema-to-fields"

type Props = {
  title: string
  /** 묶음 스위치 필드. 있으면 제목 자리에 체크박스로 그린다 */
  toggle?: Field
  checked: boolean
  onToggle: (checked: boolean) => void
  disabled?: boolean
  className?: string
}

export function SectionHeader({
  title,
  toggle,
  checked,
  onToggle,
  disabled,
  className,
}: Props) {
  if (!toggle) {
    return (
      <h3
        className={cn("text-2xs text-muted-foreground font-medium", className)}
      >
        {title}
      </h3>
    )
  }
  const id = `opt-${toggle.key}`
  return (
    <div className={cn("flex items-center gap-2", className)}>
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(c) => onToggle(c === true)}
        disabled={disabled}
      />
      <label htmlFor={id} className="text-sm font-medium">
        {toggle.label}
      </label>
    </div>
  )
}
