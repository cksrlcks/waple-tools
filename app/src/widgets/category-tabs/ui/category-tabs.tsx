import type { KeyboardEvent } from "react"
import {
  IconCode,
  IconFileTypePdf,
  IconPhoto,
  IconTool,
  IconTypography,
  IconVideo,
  type Icon,
} from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import type { CategoryTab } from "../lib/category-tabs"

const ICONS: Record<string, Icon> = {
  image: IconPhoto,
  pdf: IconFileTypePdf,
  video: IconVideo,
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

/** 제목 막대 안의 카테고리 탭 (브라우저식: 선택 탭이 아래 본문과 이어진다) */
export function CategoryTabs({
  tabs,
  value,
  onChange,
  locked = false,
  className,
}: Props) {
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (locked || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return
    const i = tabs.findIndex((t) => t.key === value)
    const next =
      tabs[(i + (e.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length]
    if (next) onChange(next.key)
  }

  return (
    <div
      role="tablist"
      data-tauri-drag-region
      aria-label="도구 종류"
      onKeyDown={onKeyDown}
      className={cn("flex h-full items-end gap-0.5 pl-2.5", className)}
    >
      {tabs.map((t) => {
        const active = t.key === value
        const Icon = ICONS[t.key] ?? IconTool
        const blocked = locked && !active
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
              title={blocked ? "작업이 끝난 뒤 이동할 수 있습니다." : undefined}
              onClick={() => onChange(t.key)}
              className={cn(
                "focus-visible:ring-primary/40 relative flex items-center gap-1.5 text-sm whitespace-nowrap outline-none focus-visible:ring-2",
                active
                  ? "tab-curves bg-background text-foreground shadow-tab h-8 rounded-t-[10px] px-3.5 font-bold"
                  : "text-muted-foreground hover:bg-foreground/6 hover:text-foreground mb-0.5 h-7 rounded-md px-3.5 transition-colors",
                blocked && "opacity-60",
              )}
            >
              <Icon
                className={cn("size-4", active && "text-primary")}
                stroke={1.75}
              />
              {/* 굵은 글자 폭을 미리 잡아 두어 선택이 바뀌어도 탭 폭이 그대로다 */}
              <span
                data-label={t.label}
                className="flex flex-col after:invisible after:h-0 after:overflow-hidden after:font-bold after:content-[attr(data-label)]"
              >
                {t.label}
              </span>
            </button>
          </div>
        )
      })}
    </div>
  )
}
