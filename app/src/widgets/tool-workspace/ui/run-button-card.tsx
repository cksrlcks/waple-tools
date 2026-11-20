import { IconArrowUpRight, IconCircleCheckFilled } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

type Props = {
  label: string
  note?: string
  /** 마지막으로 실행한 값 */
  active: boolean
  pending: boolean
  onClick: () => void
  className?: string
}

/** `x-run-buttons` 값 하나: 누르면 실행하는 카드 (이름 · 설명 한 줄) */
export function RunButtonCard({
  label,
  note,
  active,
  pending,
  onClick,
  className,
}: Props) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={pending}
      aria-current={active ? "true" : undefined}
      className={cn(
        "group bg-card hover:border-primary/60 focus-visible:ring-primary/40 flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors outline-none focus-visible:ring-2 disabled:opacity-60",
        active && "border-primary bg-primary/5",
        className,
      )}
    >
      <span className="flex w-full items-center gap-1.5">
        <span className="text-sm font-semibold">{label}</span>
        {active && (
          <IconCircleCheckFilled className="text-primary size-3.5 shrink-0" />
        )}
        <IconArrowUpRight
          className="text-muted-foreground group-hover:text-primary ml-auto size-4 shrink-0"
          stroke={1.75}
        />
      </span>
      {note && <span className="text-muted-foreground text-xs">{note}</span>}
    </button>
  )
}
