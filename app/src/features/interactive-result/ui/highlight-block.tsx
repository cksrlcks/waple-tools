import { IconCopy } from "@tabler/icons-react"

import type { Span } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { clipSpans } from "../lib/clip-text"
import { copyText } from "../model/copy-text"
import { SpanText } from "./span-text"

/** 화면에 그리는 최대 글자 수 */
const SHOWN = 20_000

type Props = {
  spans: Span[]
  className?: string
}

/** 강조 구간이 있는 글 (정규식 일치). 붙어 있는 일치도 구분되게 짝·홀 색을 번갈아 */
export function HighlightBlock({ spans, className }: Props) {
  const text = spans.map((s) => s.text).join("")
  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex min-h-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      <header className="flex h-9 shrink-0 items-center justify-end pr-1.5 pl-3">
        <Button size="sm" onClick={() => copyText(text)}>
          <IconCopy data-icon="inline-start" stroke={1.75} />
          복사
        </Button>
      </header>
      <pre className="min-h-0 flex-1 overflow-auto px-3 pb-3 font-mono text-xs break-all whitespace-pre-wrap select-text">
        <SpanText
          spans={clipSpans(spans, SHOWN)}
          markClass={(m) => (m % 2 ? "bg-primary/35" : "bg-primary/70")}
        />
      </pre>
      {text.length > SHOWN && (
        <p className="text-stage-muted shrink-0 px-3 pb-2 text-xs">
          화면에는 앞부분만 보입니다. 복사하면 전체가 복사됩니다.
        </p>
      )}
    </section>
  )
}
