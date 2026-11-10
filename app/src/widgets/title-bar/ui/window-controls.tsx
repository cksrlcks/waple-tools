import { useEffect, useState } from "react"
import { IconCopy, IconMinus, IconSquare, IconX } from "@tabler/icons-react"
import { getCurrentWindow } from "@tauri-apps/api/window"

import { cn } from "@/shared/lib/utils"

import { windowCall } from "../lib/window-call"

type Props = {
  className?: string
}

/** Windows 11 캡션 버튼 (46×32, 닫기는 빨강) */
export function WindowControls({ className }: Props) {
  const [maximized, setMaximized] = useState(false)
  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false
    const sync = async () => {
      const w = getCurrentWindow()
      setMaximized(await w.isMaximized())
      const fn = await w.onResized(async () =>
        setMaximized(await w.isMaximized()),
      )
      if (cancelled) fn()
      else unlisten = fn
    }
    // Tauri 밖에서는 getCurrentWindow가 던진다
    Promise.resolve()
      .then(sync)
      .catch(() => {})
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  const btn =
    "text-foreground/70 hover:text-foreground flex h-8 w-11.5 items-center justify-center transition-colors"
  return (
    <div className={cn("flex shrink-0 self-start", className)}>
      <button
        type="button"
        aria-label="최소화"
        onClick={() => windowCall("minimize")}
        className={cn(btn, "hover:bg-foreground/8")}
      >
        <IconMinus className="size-3.5" stroke={1.5} />
      </button>
      <button
        type="button"
        aria-label={maximized ? "이전 크기로" : "최대화"}
        onClick={() => windowCall("toggleMaximize")}
        className={cn(btn, "hover:bg-foreground/8")}
      >
        {maximized ? (
          <IconCopy className="size-3 -scale-x-100" stroke={1.5} />
        ) : (
          <IconSquare className="size-3" stroke={1.5} />
        )}
      </button>
      <button
        type="button"
        aria-label="닫기"
        onClick={() => windowCall("close")}
        className={cn(
          btn,
          "hover:bg-destructive hover:text-primary-foreground",
        )}
      >
        <IconX className="size-4" stroke={1.5} />
      </button>
    </div>
  )
}
