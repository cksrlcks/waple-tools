import { describe, expect, it } from "vitest"

import { columnCount, visibleRange } from "./grid-virtual"

describe("columnCount", () => {
  it("카드 최소 폭과 간격으로 들어가는 열 수", () => {
    // 88 + 12 = 100 한 칸
    expect(columnCount(400, 88, 12)).toBe(4)
    expect(columnCount(387, 88, 12)).toBe(3)
    expect(columnCount(388, 88, 12)).toBe(4)
  })
  it("폭이 좁거나 0이어도 1열", () => {
    expect(columnCount(0, 88, 12)).toBe(1)
    expect(columnCount(50, 88, 12)).toBe(1)
  })
})

describe("visibleRange", () => {
  it("보이는 줄 ± 여유 줄", () => {
    expect(
      visibleRange({
        scrollTop: 1000,
        viewport: 500,
        rowHeight: 100,
        rows: 100,
        overscan: 2,
      }),
    ).toEqual({ start: 8, end: 17 })
  })
  it("처음과 끝에서 줄 범위를 넘지 않는다", () => {
    expect(
      visibleRange({
        scrollTop: 0,
        viewport: 300,
        rowHeight: 100,
        rows: 2,
        overscan: 3,
      }),
    ).toEqual({ start: 0, end: 2 })
    expect(
      visibleRange({
        scrollTop: 99999,
        viewport: 300,
        rowHeight: 100,
        rows: 10,
        overscan: 1,
      }),
    ).toEqual({ start: 10, end: 10 })
  })
  it("줄이 없으면 빈 범위", () => {
    expect(
      visibleRange({
        scrollTop: 0,
        viewport: 300,
        rowHeight: 100,
        rows: 0,
        overscan: 2,
      }),
    ).toEqual({ start: 0, end: 0 })
  })
})
