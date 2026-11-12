import { IconChevronLeft, IconChevronRight } from "@tabler/icons-react"

import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Slider } from "@/shared/ui/slider"

import { useMediaProbe } from "../api/use-media-probe"
import { formatTime } from "../lib/format-time"

type Props = {
  toolId: string
  path: string
  /** 고른 시각 (초) */
  value: number
  onChange: (secs: number) => void
  disabled?: boolean
  className?: string
}

const stepButton =
  "focus-visible:ring-primary text-stage-muted hover:text-stage-foreground flex size-6 shrink-0 items-center justify-center rounded outline-none focus-visible:ring-2 disabled:pointer-events-none disabled:opacity-50"

/** 작업대의 영상 탐색 막대 (`x-seek`): 한 프레임 단위로 시각을 고른다. ←/→도 한 프레임 */
export function MediaSeekBar({
  toolId,
  path,
  value,
  onChange,
  disabled,
  className,
}: Props) {
  const probe = useMediaProbe(toolId, path)
  const box = cn(
    "flex h-11 shrink-0 items-center gap-2 px-4 text-xs",
    className,
  )

  if (probe.isError) {
    return (
      <p className={cn(box, "text-destructive")}>
        {toUserMessage(probe.error)}
      </p>
    )
  }
  if (!probe.data) {
    return (
      <p className={cn(box, "text-stage-muted")}>
        {probe.data === null
          ? "영상 길이를 알 수 없어 탐색 막대를 쓸 수 없습니다."
          : "영상 정보를 읽는 중…"}
      </p>
    )
  }

  const { duration, fps } = probe.data
  // 밀리초로 맞춘다: 도구는 1ms 오차를 허용하므로 k/fps를 반올림해도 k번째 프레임
  const clean = (s: number) =>
    Math.round(Math.min(duration, Math.max(0, s)) * 1000) / 1000
  const frame = Math.round(value * fps)
  const moveFrames = (n: number) => onChange(clean((frame + n) / fps))

  return (
    <div className={box}>
      <button
        type="button"
        aria-label="한 프레임 뒤로"
        title="한 프레임 뒤로"
        className={stepButton}
        disabled={disabled || frame <= 0}
        onClick={() => moveFrames(-1)}
      >
        <IconChevronLeft className="size-4" stroke={2} />
      </button>
      <Slider
        className="**:data-[slot=slider-track]:bg-stage-foreground/20 flex-1"
        min={0}
        max={duration}
        step={1 / fps}
        value={[Math.min(value, duration)]}
        onValueChange={(v) =>
          onChange(clean(typeof v === "number" ? v : (v[0] ?? 0)))
        }
        disabled={disabled}
        thumbLabel="추출할 프레임 위치"
      />
      <button
        type="button"
        aria-label="한 프레임 앞으로"
        title="한 프레임 앞으로"
        className={stepButton}
        disabled={disabled || value >= duration}
        onClick={() => moveFrames(1)}
      >
        <IconChevronRight className="size-4" stroke={2} />
      </button>
      <span className="text-stage-muted shrink-0 tabular-nums">
        {formatTime(value, true)} / {formatTime(duration)}
      </span>
    </div>
  )
}
