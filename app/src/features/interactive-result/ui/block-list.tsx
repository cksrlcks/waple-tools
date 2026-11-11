import type { Block } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { formatNumber } from "../lib/clip-text"
import { CodeBlock } from "./code-block"

type Props = {
  blocks: Block[]
  className?: string
}

/** 결과 블록을 차례로 그린다: 안내 글 → 숫자 → 색 → 코드 */
// ponytail: image·imagePair 블록은 아직 쓰는 도구가 없어 그리지 않는다
export function BlockList({ blocks, className }: Props) {
  const texts = blocks.filter((b) => b.type === "text")
  const numbers = blocks.filter((b) => b.type === "number")
  const colors = blocks.filter((b) => b.type === "color")
  const codes = blocks.filter((b) => b.type === "code")
  return (
    <div className={cn("flex min-h-0 flex-col gap-3", className)}>
      {texts.map((b, i) => (
        <p
          key={`t${i}`}
          className="bg-stage-foreground/8 rounded-lg px-3 py-2 text-xs"
        >
          {b.text}
        </p>
      ))}
      {(numbers.length > 0 || colors.length > 0) && (
        <dl className="flex flex-wrap gap-x-6 gap-y-1 text-xs">
          {numbers.map((b, i) => (
            <div key={`n${i}`} className="flex gap-2">
              <dt className="text-stage-muted">{b.label}</dt>
              <dd className="tabular-nums">{formatNumber(b.value, b.unit)}</dd>
            </div>
          ))}
          {colors.map((b, i) => (
            <div key={`c${i}`} className="flex items-center gap-2">
              <span
                className="size-4 rounded border"
                style={{ backgroundColor: b.hex }}
              />
              <dd className="font-mono">{b.hex}</dd>
            </div>
          ))}
        </dl>
      )}
      {codes.map((b, i) => (
        <CodeBlock
          key={`k${i}`}
          code={b.code}
          lang={b.lang}
          className="flex-1"
        />
      ))}
    </div>
  )
}
