import type { Block, Span } from "@/entities/tool"

/**
 * 강조 블록 중 조각을 이은 글이 어떤 편집기의 글과 같으면 그 편집기 안에 강조를 그리고 블록 목록에서는 뺀다.
 * 입력이 바뀌어 결과가 아직 옛 글이면 짝이 없어 결과 쪽에 그대로 남는다.
 */
export function editorMarks(
  blocks: Block[],
  values: Record<string, string>,
): { marks: Record<string, Span[]>; rest: Block[] } {
  const marks: Record<string, Span[]> = {}
  const rest = blocks.filter((b) => {
    if (b.type !== "highlight") return true
    const text = b.spans.map((s) => s.text).join("")
    const key = Object.keys(values).find(
      (k) => values[k] === text && !(k in marks),
    )
    if (key === undefined) return true
    marks[key] = b.spans
    return false
  })
  return { marks, rest }
}
