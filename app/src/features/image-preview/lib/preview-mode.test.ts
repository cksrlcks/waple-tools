import { describe, expect, it } from "vitest"

import { previewMode } from "./preview-mode"

describe("previewMode", () => {
  it("스키마가 결과만 보여 달라고 하면 result", () => {
    expect(previewMode({ "x-preview": "result" })).toBe("result")
  })
  it("그 밖에는 비교", () => {
    expect(previewMode({})).toBe("compare")
    expect(previewMode(null)).toBe("compare")
    expect(previewMode({ "x-preview": "zzz" })).toBe("compare")
  })
})
