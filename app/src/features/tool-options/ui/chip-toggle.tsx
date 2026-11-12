import { cn } from "@/shared/lib/utils"

type Props = {
  /** 칩 글자 (예: `i`) */
  short: string
  /** 풍선 설명 (옵션 이름) */
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
  className?: string
}

/** 한 글자 켜기/끄기 칩 (`x-short` 불리언 옵션, 예: 정규식 플래그) */
export function ChipToggle({
  short,
  label,
  checked,
  onChange,
  className,
}: Props) {
  return (
    <button
      type="button"
      aria-pressed={checked}
      aria-label={label}
      title={label}
      onClick={() => onChange(!checked)}
      className={cn(
        "focus-visible:ring-primary/40 inline-flex size-7 items-center justify-center rounded-full border font-mono text-xs transition-colors outline-none focus-visible:ring-2",
        checked
          ? "border-primary bg-primary/10 text-primary font-semibold"
          : "bg-card text-muted-foreground hover:bg-muted",
        className,
      )}
    >
      {short}
    </button>
  )
}
