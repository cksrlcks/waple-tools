import { describe, expect, it } from "vitest"

import { clipText, formatNumber } from "./clip-text"

describe("결과 블록 표시", () => {
  it("긴 글은 앞부분만", () => {
    expect(clipText("abcdef", 10)).toBe("abcdef")
    expect(clipText("abcdef", 3)).toBe("abc…")
  })

  it("숫자와 단위", () => {
    expect(formatNumber(1234.5, "KB")).toBe("1,234.5 KB")
    expect(formatNumber(3, null)).toBe("3")
  })
})
