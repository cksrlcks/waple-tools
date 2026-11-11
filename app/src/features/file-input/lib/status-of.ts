import type { FileResult, PathStat } from "@/entities/tool"
import { formatBytes } from "@/shared/lib/format-bytes"

/** 입력 카드 아래 한 줄 상태: 크기·폴더, 실행 뒤에는 결과 */
export function statusOf(
  item: PathStat,
  r?: FileResult,
): { text: string; tone?: "error" | "done" } {
  // 실행 결과가 먼저: 형식을 바꾸면(ttf → woff2) 원본은 original/로 옮겨져 원래 경로에 없다
  if (!r) {
    if (!item.exists) return { text: "찾을 수 없음", tone: "error" }
    return { text: item.isDir ? "폴더" : formatBytes(item.size) }
  }
  switch (r.status) {
    case "done":
      return { text: "완료", tone: "done" }
    case "skipped":
      return { text: `건너뜀(${r.reason})` }
    case "failed":
      return { text: r.message, tone: "error" }
    case "cancelled":
      return { text: "취소됨" }
  }
}
