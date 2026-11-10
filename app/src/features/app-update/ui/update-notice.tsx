import { useEffect, useLayoutEffect, useRef, useState } from "react"
import {
  IconCircleCheckFilled,
  IconExclamationCircleFilled,
  IconRocket,
  IconX,
} from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Progress } from "@/shared/ui/progress"

import { COVER } from "../config/cover"
import { dismissUpdate, installUpdate, useUpdater } from "../model/updater"

type Props = {
  /** 작업 중이면 설치하지 않는다 */
  locked?: boolean
  className?: string
}

/** 우하단 업데이트 알림 (u-drive와 같은 카드). 새 버전·진행률·결과를 보여 준다 */
export function UpdateNotice({ locked = false, className }: Props) {
  const s = useUpdater()
  useEffect(() => {
    if (s.kind !== "none") return
    const t = setTimeout(dismissUpdate, 3000)
    return () => clearTimeout(t)
  }, [s.kind])

  // 노트가 세 줄을 넘을 때만 펼치기 버튼을 보인다
  const [open, setOpen] = useState(false)
  const [clamped, setClamped] = useState(false)
  const notesRef = useRef<HTMLParagraphElement>(null)
  const notes = s.kind === "available" ? s.notes : undefined
  useLayoutEffect(() => {
    const el = notesRef.current
    setClamped(!!el && el.scrollHeight > el.clientHeight)
  }, [notes])

  if (s.kind === "idle" || s.kind === "checking") return null
  return (
    <div
      className={cn(
        "bg-popover shadow-dialog animate-in fade-in-0 w-72 rounded-xl p-3 text-sm duration-100",
        className,
      )}
    >
      {s.kind === "available" && (
        <div className="flex flex-col gap-3">
          <div
            className="-mx-3 -mt-3 flex h-28 items-center justify-center rounded-t-xl"
            style={COVER}
          >
            <IconRocket
              className="text-primary-foreground size-10 drop-shadow-md"
              stroke={1.5}
            />
          </div>
          <div className="flex flex-col gap-1">
            <div className="font-semibold">새로운 버전으로 업데이트</div>
            <p
              ref={notesRef}
              className={cn(
                "text-muted-foreground text-xs leading-relaxed whitespace-pre-line",
                open ? "max-h-48 overflow-y-auto" : "line-clamp-3",
              )}
            >
              {s.notes || "업데이트하면 앱이 다시 시작됩니다"}
            </p>
          </div>
          <div className="flex items-center gap-1">
            {(clamped || open) && (
              <Button
                variant="link"
                size="xs"
                className="text-muted-foreground px-0"
                onClick={() => setOpen(!open)}
              >
                {open ? "접기" : "자세히 보기"}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              className="ml-auto"
              onClick={dismissUpdate}
            >
              나중에
            </Button>
            <Button
              size="sm"
              disabled={locked}
              title={
                locked ? "작업이 끝난 뒤 업데이트할 수 있습니다." : undefined
              }
              onClick={() => void installUpdate()}
            >
              업데이트
            </Button>
          </div>
        </div>
      )}
      {s.kind === "downloading" && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between text-xs">
            <span className="font-medium">업데이트 받는 중…</span>
            <span className="text-muted-foreground tabular-nums">
              {s.percent === null ? "" : `${Math.round(s.percent)}%`}
            </span>
          </div>
          <Progress value={s.percent ?? 0} />
        </div>
      )}
      {s.kind === "none" && (
        <div className="flex items-center gap-2 text-xs">
          <IconCircleCheckFilled className="text-success size-4" /> 최신
          버전입니다
        </div>
      )}
      {s.kind === "error" && (
        <div className="flex items-center gap-2 text-xs">
          <IconExclamationCircleFilled className="text-destructive size-4 shrink-0" />
          <span className="min-w-0 flex-1">{s.message}</span>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={dismissUpdate}
            aria-label="닫기"
            title="닫기"
          >
            <IconX className="size-4" />
          </Button>
        </div>
      )}
    </div>
  )
}
