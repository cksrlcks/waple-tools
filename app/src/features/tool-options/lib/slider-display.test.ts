import { describe, expect, it } from "vitest"

import { sliderDisplay, sliderTick } from "./slider-display"

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

describe("sliderTick", () => {
  const field = { min: 0.5, max: 3 }
  it("숫자 힌트의 막대 위 비율 (0~1)", () => {
    expect(sliderTick(field, 0.5)).toBe(0)
    expect(sliderTick(field, 1.75)).toBe(0.5)
    expect(sliderTick(field, 3)).toBe(1)
  })
  it("범위 밖이면 끝에 붙인다", () => {
    expect(sliderTick(field, 4)).toBe(1)
    expect(sliderTick(field, 0)).toBe(0)
  })
  it("숫자가 아니면 없음", () => {
    expect(sliderTick(field, undefined)).toBeUndefined()
    expect(sliderTick(field, "2")).toBeUndefined()
    expect(sliderTick(field, Number.NaN)).toBeUndefined()
  })
})
