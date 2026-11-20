import { describe, expect, it } from "vitest"

import { downloadPercent } from "./download-percent"
import { progressText } from "./progress-text"

describe("downloadPercent", () => {
  it("받은 비율", () => {
    expect(downloadPercent({ received: 50, total: 200, step: null })).toBe(25)
    expect(downloadPercent({ received: 9, total: 3, step: null })).toBe(100)
  })
  it("전체를 모르면 null", () => {
    expect(downloadPercent(null)).toBeNull()
    expect(downloadPercent({ received: 5, total: null, step: null })).toBeNull()
  })
  it("기본 리소스 단계는 되감지 않고 null", () => {
    const step = { index: 2, count: 4, label: "heroicons" }
    expect(downloadPercent({ received: 1, total: 4, step })).toBeNull()
    expect(progressText({ received: 1, total: 4, step })).toBe(
      "기본 리소스 받는 중 2/4 (heroicons)"
    )
  })
})
