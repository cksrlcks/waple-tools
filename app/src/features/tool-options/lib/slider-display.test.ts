import { describe, expect, it } from "vitest"

import { sliderDisplay } from "./slider-display"

describe("sliderDisplay", () => {
  const field = { min: 0.5, max: 3 }
  it("값이 있으면 그 값", () => {
    expect(sliderDisplay(1.5, field, 2)).toEqual({ thumb: 1.5, badge: "1.5" })
  })
  it("null이고 힌트가 있으면 힌트 자리에 손잡이, '힌트 · 원래'", () => {
    expect(sliderDisplay(null, field, 2)).toEqual({
      thumb: 2,
      badge: "2 · 원래",
    })
  })
  it("null이고 힌트가 없거나 숫자가 아니면 '원래', 손잡이는 맨 왼쪽", () => {
    expect(sliderDisplay(null, field, undefined)).toEqual({
      thumb: 0.5,
      badge: "원래",
    })
    expect(sliderDisplay(null, field, "2")).toEqual({
      thumb: 0.5,
      badge: "원래",
    })
  })
  it("힌트가 범위 밖이면 손잡이는 범위 안, 글은 원래 값", () => {
    expect(sliderDisplay(null, field, 4)).toEqual({
      thumb: 3,
      badge: "4 · 원래",
    })
  })
})
