import { IconHourglassHigh } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

type Props = {
  /** 탭 이름 (예: PDF) */
  label: string
  className?: string
}

/** 아직 도구가 없는 탭의 임시 화면 */
export function ComingSoon({ label, className }: Props) {
  return (
    <div
      className={cn(
        "m-auto flex flex-col items-center gap-3 text-center",
        className,
      )}
    >
      <span className="bg-selection text-primary flex size-14 items-center justify-center rounded-2xl">
        <IconHourglassHigh className="size-7" stroke={1.75} />
      </span>
      <h2 className="text-lg font-bold">{label} 도구를 준비하고 있습니다</h2>
      <p className="text-muted-foreground">조금만 기다려 주세요.</p>
    </div>
  )
}
