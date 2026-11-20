import type { Block } from "@/entities/tool"

/** `optionHints` 블록(그리지 않음)의 값을 모은다: 값이 null인 옵션 칸에 보여 줄 값 */
export function optionHints(blocks: Block[] | undefined) {
  const out: Record<string, unknown> = {}
  for (const b of blocks ?? [])
    if (b.type === "optionHints") Object.assign(out, b.values)
  return out
}

/** `optionHints` 블록의 `hidden`을 모은다: 지금 결과에서 바꿀 수 없어 패널에서 감출 옵션 키 */
export function hiddenOptions(blocks: Block[] | undefined): string[] {
  const out = new Set<string>()
  for (const b of blocks ?? [])
    if (b.type === "optionHints") for (const k of b.hidden ?? []) out.add(k)
  return [...out]
}
