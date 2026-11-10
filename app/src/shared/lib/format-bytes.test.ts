import { describe, expect, it } from "vitest"

import { formatBytes, formatChange } from "./format-bytes"

describe("formatBytes", () => {
  it("1024 단위, 소수 한 자리", () => {
    expect(formatBytes(0)).toBe("0 B")
    expect(formatBytes(1536)).toBe("1.5 KB")
    expect(formatBytes(47396249)).toBe("45.2 MB")
  })
})

describe("formatChange", () => {
  it("감소는 -퍼센트", () => {
    expect(formatChange(47396249, 12687769)).toBe("45.2 MB → 12.1 MB (-73%)")
  })
  it("증가는 +퍼센트", () => {
    expect(formatChange(100, 150)).toBe("100 B → 150 B (+50%)")
  })
  it("이전 크기가 0이면 퍼센트 생략", () => {
    expect(formatChange(0, 10)).toBe("0 B → 10 B")
  })
})
