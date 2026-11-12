import type { DiffLine } from "@/entities/tool"

/** 좌우 비교의 한 행. 짝이 없는 쪽은 비어 있다 */
export type SplitRow = { left?: DiffLine; right?: DiffLine }

/** 이어진 delete 묶음과 바로 뒤 insert 묶음을 한 줄씩 짝짓는다. 같은 줄은 양쪽에 */
export function toSplitRows(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = []
  let i = 0
  while (i < lines.length) {
    if (lines[i].tag === "equal") {
      rows.push({ left: lines[i], right: lines[i] })
      i++
      continue
    }
    const dels: DiffLine[] = []
    const ins: DiffLine[] = []
    while (i < lines.length && lines[i].tag === "delete") dels.push(lines[i++])
    while (i < lines.length && lines[i].tag === "insert") ins.push(lines[i++])
    for (let k = 0; k < Math.max(dels.length, ins.length); k++) {
      rows.push({ left: dels[k], right: ins[k] })
    }
  }
  return rows
}
