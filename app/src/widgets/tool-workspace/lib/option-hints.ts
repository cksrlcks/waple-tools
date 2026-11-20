import type { Block } from "@/entities/tool"

/** `optionHints` 블록(그리지 않음)의 값을 모은다: 값이 null인 옵션 칸에 보여 줄 값 */
export function optionHints(blocks: Block[] | undefined) {
  const out: Record<string, unknown> = {}
  for (const b of blocks ?? [])
    if (b.type === "optionHints") Object.assign(out, b.values)
  return out
}
