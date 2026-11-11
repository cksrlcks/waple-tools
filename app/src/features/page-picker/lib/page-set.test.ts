import { describe, expect, it } from "vitest"

import { formatPages, NO_PAGES, parsePages } from "./page-set"

const set = (...n: number[]) => new Set(n)

describe("parsePages", () => {
  it("빈 칸은 전체, NO_PAGES는 없음", () => {
    expect(parsePages("", 3)).toEqual(set(1, 2, 3))
    expect(parsePages(NO_PAGES, 3)).toEqual(set())
  })
  it("범위·열린 범위·범위 밖", () => {
    expect(parsePages("1-2, 4, 6-", 7)).toEqual(set(1, 2, 4, 6, 7))
    expect(parsePages("-2,9", 5)).toEqual(set(1, 2))
    expect(parsePages("x,3", 5)).toEqual(set(3))
  })
})

describe("formatPages", () => {
  it("연속은 묶고, 전부면 빈 칸, 없으면 NO_PAGES", () => {
    expect(formatPages(set(5, 1, 2, 3, 7), 9)).toBe("1-3,5,7")
    expect(formatPages(set(1, 2, 3), 3)).toBe("")
    expect(formatPages(set(), 3)).toBe(NO_PAGES)
  })
  it("parse와 왕복", () => {
    expect(formatPages(parsePages("2-4,8", 10), 10)).toBe("2-4,8")
  })
})
