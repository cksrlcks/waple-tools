import { IconColorPicker } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"

type Props = {
  id?: string
  /** 칸 이름 라벨 id: 견본 묶음의 접근성 이름 */
  labelId?: string
  value: string
  swatches: string[]
  onChange: (value: string) => void
  disabled?: boolean
  className?: string
}

const slot =
  "size-6 shrink-0 cursor-pointer rounded-full border border-border disabled:cursor-not-allowed disabled:opacity-50"
const picked = "ring-2 ring-primary ring-offset-2 ring-offset-background"
const focus =
  "focus-visible:outline-1 focus-visible:-outline-offset-1 focus-visible:outline-primary/60"

/**
 * 견본 버튼들 + 사용자 지정 1칸(색 입력). 견본에 없는 값이면 사용자 지정 칸이 그 색으로 선택 상태,
 * 견본 값이면 사용자 지정 칸에 스포이트 아이콘을 얹어 견본과 구분한다.
 */
export function SwatchField({
  id,
  labelId,
  value,
  swatches,
  onChange,
  disabled,
  className,
}: Props) {
  const isSwatch = swatches.some((c) => c.toLowerCase() === value.toLowerCase())
  return (
    <div
      id={id}
      role="group"
      aria-labelledby={labelId}
      className={cn("flex items-center gap-2", className)}
    >
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
            className={cn(slot, focus, on && picked)}
            style={{ backgroundColor: c }}
            onClick={() => onChange(c)}
          />
        )
      })}
      <label className="relative flex" title="사용자 지정">
        <input
          type="color"
          aria-label={`사용자 지정 색 ${value}${isSwatch ? "" : " (선택됨)"}`}
          disabled={disabled}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className={cn(
            slot,
            focus,
            "peer appearance-none bg-transparent p-0 [&::-webkit-color-swatch]:rounded-full [&::-webkit-color-swatch]:border-none [&::-webkit-color-swatch-wrapper]:p-0",
            !isSwatch && picked,
          )}
        />
        {/* 얹은 아이콘이 입력의 포커스 표시를 가리지 않게 같은 포커스 표시를 아이콘에도 */}
        {isSwatch && (
          <span
            aria-hidden
            className="bg-background text-muted-foreground border-border peer-focus-visible:outline-primary/60 pointer-events-none absolute inset-0 flex items-center justify-center rounded-full border peer-disabled:opacity-50 peer-focus-visible:outline-1 peer-focus-visible:-outline-offset-1"
          >
            <IconColorPicker className="size-3.5" stroke={1.75} />
          </span>
        )}
      </label>
    </div>
  )
}
