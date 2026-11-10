import type { ReactNode } from "react"

import logoFull from "@/shared/assets/logo-full.svg"
import { cn } from "@/shared/lib/utils"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip"

import { AppMenu } from "./app-menu"
import { WindowControls } from "./window-controls"

type Props = {
  /** 아랫줄에 들어갈 탭 막대 */
  children?: ReactNode
  onOpenSettings: () => void
  /** 로고를 누르면 처음 화면으로 */
  onHome: () => void
  locked?: boolean
  className?: string
}

/** 앱이 직접 그리는 창 제목 막대: 윗줄 로고 · 메뉴 · 창 버튼, 아랫줄 탭. 빈 곳을 끌면 창이 움직인다 */
export function TitleBar({
  children,
  onOpenSettings,
  onHome,
  locked,
  className,
}: Props) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "bg-titlebar to-foreground/4 flex shrink-0 flex-col bg-linear-to-r from-transparent select-none",
        className,
      )}
    >
      <div data-tauri-drag-region className="flex h-10 items-center pl-2">
        <div data-tauri-drag-region className="flex flex-1 items-center gap-1">
          <Tooltip>
            <TooltipTrigger
              onClick={onHome}
              disabled={locked}
              aria-label="홈"
              className="hover:bg-muted dark:hover:bg-muted/50 focus-visible:ring-ring/50 flex h-7 items-center rounded-md px-2 outline-none focus-visible:ring-2 disabled:pointer-events-none"
            >
              <img
                src={logoFull}
                alt=""
                className="h-3.5 dark:invert"
                draggable={false}
              />
            </TooltipTrigger>
            <TooltipContent side="bottom">홈</TooltipContent>
          </Tooltip>
          <AppMenu onOpenSettings={onOpenSettings} locked={locked} />
        </div>
        <WindowControls />
      </div>
      <div data-tauri-drag-region className="flex h-9 min-w-0 items-end">
        {children}
      </div>
    </header>
  )
}
