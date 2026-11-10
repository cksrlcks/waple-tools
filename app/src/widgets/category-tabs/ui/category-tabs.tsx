import type { KeyboardEvent } from "react"
import {
  IconCode,
  IconFileTypePdf,
  IconPhoto,
  IconTool,
  IconTypography,
  type Icon,
} from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import type { CategoryTab } from "../lib/category-tabs"

const ICONS: Record<string, Icon> = {
  image: IconPhoto,
  pdf: IconFileTypePdf,
  web: IconTypography,
  misc: IconTool,
  dev: IconCode,
}

type Props = {
  tabs: CategoryTab[]
  value: string
  onChange: (key: string) => void
  /** 작업 중이면 다른 탭으로 가지 못한다 */
  locked?: boolean
  className?: string
}

/** 창 맨 위 카테고리 탭 (브라우저식: 선택 탭이 아래 본문과 이어진다) */
export function CategoryTabs({
  tabs,
  value,
  onChange,
  locked = false,
  className,
}: Props) {
  const usable = tabs.filter((t) => !t.disabled)
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (locked || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return
    const i = usable.findIndex((t) => t.key === value)
    const next =
      usable[
        (i + (e.key === "ArrowRight" ? 1 : -1) + usable.length) % usable.length
      ]
    if (next) onChange(next.key)
  }

  return (
    <div
      role="tablist"
      aria-label="도구 종류"
      onKeyDown={onKeyDown}
      className={cn(
        "bg-titlebar flex h-10 shrink-0 items-end gap-1 px-2 pt-1.5",
        className,
      )}
    >
      {tabs.map((t) => {
        const active = t.key === value
        const Icon = ICONS[t.key] ?? IconTool
        const blocked = t.disabled || (locked && !active)
        return (
          <div key={t.key} className="flex items-end">
            {t.key === "dev" && (
              <span className="bg-border mx-1 mb-2.5 h-4 w-px" />
            )}
            <button
              type="button"
              role="tab"
              aria-selected={active}
              tabIndex={active ? 0 : -1}
              disabled={blocked}
              title={
                t.disabled
                  ? "준비 중"
                  : locked && !active
                    ? "작업이 끝난 뒤 이동할 수 있습니다."
                    : undefined
              }
              onClick={() => onChange(t.key)}
              className={cn(
                "focus-visible:ring-primary/40 relative flex items-center gap-1.5 text-sm whitespace-nowrap outline-none focus-visible:ring-2",
                active
                  ? "tab-curves bg-background text-foreground h-8.5 rounded-t-lg px-3.5 font-medium"
                  : "text-muted-foreground hover:bg-foreground/8 hover:text-foreground mb-0.75 h-7 rounded-md px-3 transition-colors",
                t.disabled && "opacity-40",
                locked && !active && !t.disabled && "opacity-60",
              )}
            >
              <Icon className="size-4" stroke={1.75} />
              {t.label}
            </button>
          </div>
        )
      })}
    </div>
  )
}
