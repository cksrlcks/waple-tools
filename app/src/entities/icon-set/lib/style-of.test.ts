import { describe, expect, it } from "vitest"

import { RECOMMENDED } from "../config/recommended"
import { TEST_SET } from "./test-set"
import { styleOf, stylesOf } from "./style-of"

describe("styleOf", () => {
  const hero = RECOMMENDED.find((r) => r.prefix === "heroicons")!.styles!
  it("가장 긴 접미 키를 고른다", () => {
    expect(styleOf("academic-cap-20-solid", hero)).toBe("20-solid")
    expect(styleOf("academic-cap-solid", hero)).toBe("solid")
    expect(styleOf("academic-cap", hero)).toBe("")
    expect(styleOf("home-filled", { "": "선", filled: "채움" })).toBe("filled")
  })
  it("stylesOf: 추천 → suffixes → {}", () => {
    expect(stylesOf({ ...TEST_SET, prefix: "other", suffixes: { line: "Line" } })).toEqual({ line: "Line" })
    expect(stylesOf({ ...TEST_SET, prefix: "other" })).toEqual({})
    expect(stylesOf({ ...TEST_SET, prefix: "ri" })).toEqual({ line: "선", fill: "채움" })
  })
})
