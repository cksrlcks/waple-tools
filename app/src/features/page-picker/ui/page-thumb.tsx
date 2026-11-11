import { IconCheck } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

type Props = {
  url: string
  page: number
  selected: boolean
  /** false면 보기만 한다 (고르기 표시 없음) */
  selectable?: boolean
  /** shift: Shift를 누른 채 눌렀는가 (범위 선택) */
  onToggle: (shift: boolean) => void
  disabled?: boolean
  className?: string
}

/** 페이지 썸네일 한 장. 누르면 고르기/빼기 */
export function PageThumb({
  url,
  page,
  selected,
  selectable = true,
  onToggle,
  disabled,
  className,
}: Props) {
  return (
    <button
      type="button"
      aria-pressed={selectable ? selected : undefined}
      aria-label={`${page}쪽`}
      onClick={(e) => onToggle(e.shiftKey)}
      disabled={disabled || !selectable}
      className={cn(
        "group focus-visible:ring-primary flex flex-col items-center gap-1.5 rounded-lg outline-none focus-visible:ring-2",
        selectable && "disabled:opacity-60",
        className,
      )}
    >
      <span
        className={cn(
          "relative block overflow-hidden rounded-md ring-2 transition-shadow",
          !selectable
            ? "ring-transparent"
            : selected
              ? "ring-primary"
              : "group-hover:ring-stage-foreground/25 ring-transparent",
        )}
      >
        <img
          src={url}
          alt=""
          className={cn(
            "block max-h-44 w-auto transition-opacity",
            selectable && !selected && "opacity-40",
          )}
        />
        {selectable && selected && (
          <span className="bg-primary text-primary-foreground absolute top-1.5 right-1.5 flex size-5 items-center justify-center rounded-full">
            <IconCheck className="size-3.5" stroke={2.5} />
          </span>
        )}
      </span>
      <span
        className={cn(
          "text-xs tabular-nums",
          selectable && !selected
            ? "text-stage-muted"
            : "text-stage-foreground",
        )}
      >
        {page}
      </span>
    </button>
  )
}
