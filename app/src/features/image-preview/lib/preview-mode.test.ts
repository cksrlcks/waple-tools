import { describe, expect, it } from "vitest"

import { previewMode } from "./preview-mode"

describe("previewMode", () => {
  it("스키마가 결과만 보여 달라고 하면 result", () => {
    expect(previewMode({ "x-preview": "result" })).toBe("result")
  })
  it("미리보기가 없는 도구는 none", () => {
    expect(previewMode({ "x-preview": "none" })).toBe("none")
  })
  it("그 밖에는 비교", () => {
    expect(previewMode({})).toBe("compare")
    expect(previewMode(null)).toBe("compare")
    expect(previewMode({ "x-preview": "zzz" })).toBe("compare")
  })
  it("x-preview-result-when: 옵션이 맞을 때만 결과만", () => {
    const schema = { "x-preview-result-when": { resize: [true] } }
    expect(previewMode(schema, { resize: true })).toBe("result")
    expect(previewMode(schema, { resize: false })).toBe("compare")
    expect(previewMode(schema)).toBe("compare")
  })
})
