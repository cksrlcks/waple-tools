import { describe, expect, it } from "vitest"

import { isEnabled } from "./is-enabled"
import type { Field } from "./schema-to-fields"

const field = (enabledWhen?: Record<string, unknown[]>[]): Field => ({
  kind: "checkbox",
  key: "x",
  label: "x",
  default: false,
  ...(enabledWhen ? { enabledWhen } : {}),
})

describe("isEnabled", () => {
  it("조건이 없으면 항상 활성", () => {
    expect(isEnabled(field(), {})).toBe(true)
  })
  it("AND: 모든 조건이 맞아야 활성", () => {
    const f = field([{ compress: [true], mode: ["quality"] }])
    expect(isEnabled(f, { compress: true, mode: "quality" })).toBe(true)
    expect(isEnabled(f, { compress: true, mode: "targetKb" })).toBe(false)
    expect(isEnabled(f, { compress: false, mode: "quality" })).toBe(false)
  })
  it("OR: 조건 묶음 중 하나라도 맞으면 활성", () => {
    const f = field([{ format: ["jpg"] }, { fit: ["pad"] }])
    expect(isEnabled(f, { format: "png", fit: "pad" })).toBe(true)
    expect(isEnabled(f, { format: "jpg", fit: "cover" })).toBe(true)
    expect(isEnabled(f, { format: "png", fit: "cover" })).toBe(false)
  })
  it("허용 값 목록 중 하나면 맞음", () => {
    const f = field([{ size: ["width", "longEdge", "exact"] }])
    expect(isEnabled(f, { size: "longEdge" })).toBe(true)
    expect(isEnabled(f, { size: "height" })).toBe(false)
  })
  it("값이 목록이면 하나라도 맞으면 활성 (넣은 파일 확장자)", () => {
    const f = field([{ $inputExt: ["png"] }])
    expect(isEnabled(f, { $inputExt: ["jpg", "png"] })).toBe(true)
    expect(isEnabled(f, { $inputExt: ["jpg"] })).toBe(false)
    expect(isEnabled(f, { $inputExt: [] })).toBe(false)
  })
})
