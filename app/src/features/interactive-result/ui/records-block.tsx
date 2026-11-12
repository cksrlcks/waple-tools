import { IconCopy } from "@tabler/icons-react"

import type { RecordItem } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { recordsText } from "../lib/records-text"
import { copyText } from "../model/copy-text"
import { RecordCard } from "./record-card"

type Props = {
  items: RecordItem[]
  /** 있으면 [복사]가 카드 대신 이 글을 복사한다 */
  copy?: string
  className?: string
}

/** 카드 목록 + [복사] */
export function RecordsBlock({ items, copy, className }: Props) {
  return (
    <section className={cn("flex min-h-0 flex-col gap-2", className)}>
      <header className="flex shrink-0 items-center justify-end">
        <Button
          size="sm"
          variant="outline"
          onClick={() => copyText(copy ?? recordsText(items))}
        >
          <IconCopy data-icon="inline-start" stroke={1.75} />
          복사
        </Button>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
        {items.map((r, i) => (
          <RecordCard key={i} item={r} className="shrink-0" />
        ))}
      </div>
    </section>
  )
}
