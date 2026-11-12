import type { RecordItem } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  item: RecordItem
  className?: string
}

/** `records` 블록의 카드 하나: 이름표 · 보조 글, 본문, 이름·값 줄 */
export function RecordCard({ item, className }: Props) {
  const fields = item.fields ?? []
  return (
    <article
      className={cn(
        "bg-card text-card-foreground flex flex-col gap-2 rounded-lg border p-3",
        className,
      )}
    >
      <header className="flex items-center justify-between gap-3">
        <span className="bg-muted rounded-md px-2 py-0.5 text-xs font-medium">
          {item.title}
        </span>
        {item.meta && (
          <span className="text-muted-foreground font-mono text-xs tabular-nums">
            {item.meta}
          </span>
        )}
      </header>
      <p className="font-mono text-sm break-all whitespace-pre-wrap select-text">
        {item.text || (
          <span className="text-muted-foreground text-xs">(빈 일치)</span>
        )}
      </p>
      {fields.length > 0 && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 border-t border-dashed pt-2 font-mono text-xs">
          {fields.map((f, i) => (
            <div key={i} className="contents">
              <dt className="text-muted-foreground">{f.label}</dt>
              <dd className="break-all select-text">
                {f.value || <span className="text-muted-foreground">—</span>}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </article>
  )
}
