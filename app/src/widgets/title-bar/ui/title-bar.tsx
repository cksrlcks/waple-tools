import type { ReactNode } from "react"
import { IconStack2 } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

import { AppMenu } from "./app-menu"
import { WindowControls } from "./window-controls"

type Props = {
  /** 가운데 들어갈 탭 막대 */
  children?: ReactNode
  onOpenSettings: () => void
  locked?: boolean
  className?: string
}

/** 앱이 직접 그리는 창 제목 막대: 로고 · 메뉴 · 탭 · 창 버튼. 빈 곳을 끌면 창이 움직인다 */
export function TitleBar({
  children,
  onOpenSettings,
  locked,
  className,
}: Props) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "bg-titlebar flex h-10 shrink-0 items-stretch select-none",
        className,
      )}
    >
      <div
        data-tauri-drag-region
        className="flex items-center gap-1.5 pr-2 pl-3"
      >
        <span className="from-brand-a to-brand-b text-primary-foreground shadow-item flex size-5 items-center justify-center rounded-md bg-linear-to-br">
          <IconStack2 className="size-3.5" stroke={2} />
        </span>
        <AppMenu onOpenSettings={onOpenSettings} locked={locked} />
      </div>
      <span className="bg-border my-3 w-px" />
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-end">
        {children}
      </div>
      <WindowControls />
    </header>
  )
}
