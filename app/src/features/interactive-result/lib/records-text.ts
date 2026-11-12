import type { RecordItem } from "@/entities/tool"

/** 카드 목록을 복사용 글로: 카드마다 `이름표 · 보조 글` 줄, 본문, `이름: 값` 줄, 빈 줄 */
export function recordsText(items: RecordItem[]): string {
  return items
    .map((r) =>
      [
        r.meta ? `${r.title} · ${r.meta}` : r.title,
        r.text,
        ...(r.fields ?? []).map((f) => `${f.label}: ${f.value}`),
      ].join("\n"),
    )
    .join("\n\n")
}
