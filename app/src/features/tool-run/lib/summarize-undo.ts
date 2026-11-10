import type { UndoItem } from "@/entities/tool"

export function summarizeUndo(items: UndoItem[]): string {
  const skipped = items.filter((i) => i.status === "skipped").length
  return `되돌렸습니다 (처리 ${items.length - skipped}, 건너뜀 ${skipped})`
}
