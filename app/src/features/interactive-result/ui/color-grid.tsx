import { toast } from "sonner"

import type { Block } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  colors: Extract<Block, { type: "color" }>[]
  className?: string
}

const LINE =
  "hover:bg-stage-foreground/12 focus-visible:ring-primary/40 -mx-1 truncate rounded-sm px-1 text-left outline-none focus-visible:ring-2"

/** 색 칩 격자. 색 면·HEX를 누르면 HEX, rgb·hsl 줄을 누르면 그 줄을 복사한다 */
export function ColorGrid({ colors, className }: Props) {
  const copy = (value: string) =>
    navigator.clipboard
      .writeText(value)
      .then(() => toast.success(`${value} 복사했습니다.`))
      .catch(() => toast.error("복사할 수 없습니다."))
  return (
    <ul
      className={cn(
        "grid shrink-0 grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-2",
        className,
      )}
    >
      {colors.map((c, i) => (
        <li
          key={i}
          className="bg-stage-foreground/8 flex flex-col overflow-hidden rounded-lg"
        >
          {/* 색 면 자체가 데이터라 인라인 색을 쓴다. text가 있으면 그 글자색 견본. 밝은 색도 경계가 보이게 안쪽 테두리 */}
          <button
            type="button"
            title="누르면 HEX를 복사합니다"
            aria-label={`${c.label ? `${c.label} ` : ""}${c.hex} 복사`}
            onClick={() => copy(c.hex)}
            className="ring-stage-foreground/10 focus-visible:ring-primary/40 flex h-16 items-end rounded-t-lg px-2 py-1.5 text-left text-xs font-medium ring-1 outline-none ring-inset focus-visible:ring-2"
            style={{ backgroundColor: c.hex, color: c.text }}
          >
            {c.text && "본문 글자"}
          </button>
          <div className="flex min-w-0 flex-col items-start gap-0.5 px-2 py-1.5">
            <button
              type="button"
              aria-label={`${c.hex} 복사`}
              onClick={() => copy(c.hex)}
              className={cn(LINE, "font-mono text-xs")}
            >
              {c.hex}
            </button>
            {c.label && (
              <span className="text-2xs max-w-full truncate font-medium">
                {c.label}
              </span>
            )}
            {c.detail?.split(" · ").map((d) => (
              <button
                key={d}
                type="button"
                aria-label={`${d} 복사`}
                onClick={() => copy(d)}
                className={cn(
                  LINE,
                  "text-stage-muted text-2xs max-w-full tabular-nums",
                )}
              >
                {d}
              </button>
            ))}
          </div>
        </li>
      ))}
    </ul>
  )
}
