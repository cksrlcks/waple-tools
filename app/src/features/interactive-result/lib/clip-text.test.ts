import { describe, expect, it } from "vitest"

import { clipSpans, clipText, formatNumber } from "./clip-text"

describe("결과 블록 표시", () => {
  it("긴 글은 앞부분만", () => {
    expect(clipText("abcdef", 10)).toBe("abcdef")
    expect(clipText("abcdef", 3)).toBe("abc…")
  })

  it("강조 조각도 이은 길이로 자른다", () => {
    const spans = [{ text: "ab" }, { text: "cd", mark: 0 }, { text: "ef" }]
    expect(clipSpans(spans, 6)).toEqual(spans)
    expect(clipSpans(spans, 3)).toEqual([
      { text: "ab" },
      { text: "c…", mark: 0 },
    ])
  })

  it("숫자와 단위", () => {
    expect(formatNumber(1234.5, "KB")).toBe("1,234.5 KB")
    expect(formatNumber(3, null)).toBe("3")
  })
})
