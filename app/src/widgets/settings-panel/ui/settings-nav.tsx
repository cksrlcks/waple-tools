import { cn } from "@/shared/lib/utils"

import { SETTINGS_SECTIONS, type SettingsSectionId } from "../config/sections"

type Props = {
  value: SettingsSectionId
  onChange: (id: SettingsSectionId) => void
  className?: string
}

/** 설정 창 왼쪽 섹션 목록 */
export function SettingsNav({ value, onChange, className }: Props) {
  return (
    <nav className={cn("flex flex-col gap-0.5", className)}>
      {SETTINGS_SECTIONS.map(({ id, label, icon: Icon }) => {
        const selected = id === value
        return (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            aria-current={selected ? "page" : undefined}
            className={cn(
              "hover:bg-accent focus-visible:ring-primary/40 flex h-9 items-center gap-3 rounded-md px-2.5 text-left text-sm transition-colors outline-none focus-visible:ring-2",
              selected && "bg-selection hover:bg-selection font-bold",
            )}
          >
            <Icon
              className={cn(
                "size-4 shrink-0 translate-y-px",
                selected ? "text-primary" : "text-muted-foreground",
              )}
              stroke={1.75}
            />
            {label}
          </button>
        )
      })}
    </nav>
  )
}
