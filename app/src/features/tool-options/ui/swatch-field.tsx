import { cn } from "@/shared/lib/utils"

type Props = {
  id?: string
  value: string
  swatches: string[]
  onChange: (value: string) => void
  disabled?: boolean
  className?: string
}

const slot =
  "size-6 shrink-0 cursor-pointer rounded-full border border-border disabled:cursor-not-allowed disabled:opacity-50"
const picked = "ring-2 ring-primary ring-offset-2 ring-offset-background"

/** 견본 버튼들 + 사용자 지정 1칸(색 입력). 견본에 없는 값이면 사용자 지정 칸이 선택 상태 */
export function SwatchField({
  id,
  value,
  swatches,
  onChange,
  disabled,
  className,
}: Props) {
  const isSwatch = swatches.some((c) => c.toLowerCase() === value.toLowerCase())
  return (
    <div id={id} className={cn("flex items-center gap-2", className)}>
      {swatches.map((c) => {
        const on = c.toLowerCase() === value.toLowerCase()
        return (
          <button
            key={c}
            type="button"
            aria-pressed={on}
            aria-label={c}
            title={c}
            disabled={disabled}
            className={cn(slot, on && picked)}
            style={{ backgroundColor: c }}
            onClick={() => onChange(c)}
          />
        )
      })}
      <input
        type="color"
        aria-pressed={!isSwatch}
        aria-label={`사용자 지정 색 ${value}`}
        title="사용자 지정"
        disabled={disabled}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={cn(
          slot,
          "appearance-none bg-transparent p-0",
          !isSwatch && picked,
        )}
      />
    </div>
  )
}
