import type { Span } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  spans: Span[]
  /** `mark`가 있는 조각의 바탕 클래스 */
  markClass: (mark: number) => string
}

/** 조각을 이어 그린다. `mark`가 있는 조각만 바탕을 칠한다 */
export function SpanText({ spans, markClass }: Props) {
  return spans.map((s, i) =>
    s.mark == null ? (
      s.text
    ) : (
      <mark
        key={i}
        className={cn("text-stage-foreground rounded-xs", markClass(s.mark))}
      >
        {s.text}
      </mark>
    ),
  )
}
