import type { Field } from "./schema-to-fields"

export type Section = {
  /** 묶음 이름 (없으면 제목 없음) */
  title?: string
  /** 묶음 제목 자리에 그릴 스위치 필드 */
  toggle?: Field
  fields: Field[]
}

/** 필드 순서를 지키며 연속한 같은 `group`끼리 묶는다. `groupToggle` 필드는 그 묶음의 제목 스위치가 된다 */
export function toSections(fields: Field[]): Section[] {
  const out: Section[] = []
  for (const f of fields) {
    const group = "group" in f ? f.group : undefined
    let cur = out[out.length - 1]
    if (!cur || cur.title !== group) {
      cur = { ...(group ? { title: group } : {}), fields: [] }
      out.push(cur)
    }
    if ("groupToggle" in f && f.groupToggle && !cur.toggle) cur.toggle = f
    else cur.fields.push(f)
  }
  return out
}
