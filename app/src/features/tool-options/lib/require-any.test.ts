import { describe, expect, it } from "vitest"

import { requireAnyMissing } from "./require-any"

const schema = { "x-require-any": ["resize", "convert", "compress"] }

describe("requireAnyMissing", () => {
  it("하나도 켜지지 않으면 안내 문구", () => {
    expect(
      requireAnyMissing(schema, {
        resize: false,
        convert: false,
        compress: false,
      }),
    ).toBe("적용할 단계를 하나 이상 켜세요.")
  })
  it("하나라도 켜지면 null", () => {
    expect(
      requireAnyMissing(schema, {
        resize: false,
        convert: true,
        compress: false,
      }),
    ).toBeNull()
  })
  it("조건이 없는 스키마는 null", () => {
    expect(requireAnyMissing({}, {})).toBeNull()
    expect(requireAnyMissing(null, {})).toBeNull()
  })
})

describe("requireAnyMissing — x-require-change", () => {
  const schema = {
    "x-require-change": { angle: "none", flipH: false, crop: "none" },
    "x-require-message": "회전·크롭·여백 중 하나 이상 고르세요.",
  }
  it("모두 그대로면 스키마의 안내 문구", () => {
    expect(
      requireAnyMissing(schema, { angle: "none", flipH: false, crop: "none" }),
    ).toBe("회전·크롭·여백 중 하나 이상 고르세요.")
  })
  it("하나라도 바뀌면 null", () => {
    expect(
      requireAnyMissing(schema, { angle: "r90", flipH: false, crop: "none" }),
    ).toBeNull()
  })
})
