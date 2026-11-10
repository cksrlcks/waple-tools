import { describe, expect, it } from "vitest"

import { clampPercent } from "./clamp-percent"

describe("clampPercent", () => {
  it("상자 안 위치를 0~100 비율로", () => {
    expect(clampPercent(50, 200)).toBe(25)
    expect(clampPercent(-10, 200)).toBe(0)
    expect(clampPercent(500, 200)).toBe(100)
  })
  it("폭이 0이면 가운데", () => {
    expect(clampPercent(10, 0)).toBe(50)
  })
})
