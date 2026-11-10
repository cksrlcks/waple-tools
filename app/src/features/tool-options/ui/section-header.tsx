import { cn } from "@/shared/lib/utils"
import { Switch } from "@/shared/ui/switch"

import type { Field } from "../lib/schema-to-fields"

type Props = {
  title: string
  /** 묶음 스위치 필드. 있으면 제목 오른쪽에 스위치로 그린다 */
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
      <h3 className={cn("text-foreground text-xs font-semibold", className)}>
        {title}
      </h3>
    )
  }
  const id = `opt-${toggle.key}`
  return (
    <div className={cn("flex items-center justify-between gap-3", className)}>
      <label
        htmlFor={id}
        className={cn(
          "text-xs font-semibold",
          !checked && "text-muted-foreground",
        )}
      >
        {toggle.label}
      </label>
      <Switch
        id={id}
        checked={checked}
        onCheckedChange={(c) => onToggle(c === true)}
        disabled={disabled}
      />
    </div>
  )
}
