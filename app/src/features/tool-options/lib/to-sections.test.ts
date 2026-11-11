import { describe, expect, it } from "vitest"

import type { Field } from "./schema-to-fields"
import { toSections } from "./to-sections"

const cb = (key: string, extra: Partial<Field> = {}): Field =>
  ({ kind: "checkbox", key, label: key, default: false, ...extra }) as Field

describe("toSections", () => {
  it("묶음이 없으면 제목 없는 한 덩어리", () => {
    const s = toSections([cb("a"), cb("b")])
    expect(s).toHaveLength(1)
    expect(s[0].title).toBeUndefined()
    expect(s[0].fields.map((f) => f.key)).toEqual(["a", "b"])
  })
  it("연속한 같은 묶음끼리 모으고, 스위치 필드는 제목 자리로", () => {
    const s = toSections([
      cb("resize", { group: "크기", groupToggle: true }),
      cb("width", { group: "크기" }),
      cb("compress", { group: "용량", groupToggle: true }),
      cb("quality", { group: "용량" }),
      cb("metadata", { group: "저장" }),
    ])
    expect(s.map((x) => x.title)).toEqual(["크기", "용량", "저장"])
    expect(s[0].toggle?.key).toBe("resize")
    expect(s[0].fields.map((f) => f.key)).toEqual(["width"])
    expect(s[2].toggle).toBeUndefined()
  })
  it("묶음 안내 문구(groupNote)는 그 묶음 제목 아래로", () => {
    const s = toSections([
      cb("a", { group: "세트", groupNote: "안내" }),
      cb("b", { group: "세트" }),
      cb("c", { group: "모양" }),
    ])
    expect(s[0].note).toBe("안내")
    expect(s[0].fields).toHaveLength(2)
    expect(s[1].note).toBeUndefined()
  })
})
