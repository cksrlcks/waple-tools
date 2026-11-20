import { describe, expect, it } from "vitest"

import { parseSliderInput } from "./parse-slider-input"

describe("parseSliderInput", () => {
  const size = { min: 8, max: 512, step: 1, nullable: true }
  const stroke = { min: 0.5, max: 3, step: 0.25 }
  it("범위 안 숫자는 그대로", () => {
    expect(parseSliderInput("48", size)).toBe(48)
    expect(parseSliderInput(" 1.5 ", stroke)).toBe(1.5)
  })
  it("범위 밖이면 끝 값으로", () => {
    expect(parseSliderInput("4", size)).toBe(8)
    expect(parseSliderInput("9999", size)).toBe(512)
    expect(parseSliderInput("-1", stroke)).toBe(0.5)
  })
  it("단위에 맞춰 반올림 (정수 칸에 소수를 넣어도 정수)", () => {
    expect(parseSliderInput("24.6", size)).toBe(25)
    expect(parseSliderInput("1.6", stroke)).toBe(1.5)
    expect(parseSliderInput("1.2", stroke)).toBe(1.25)
  })
  it("소수점 쉼표도 받는다", () => {
    expect(parseSliderInput("1,5", stroke)).toBe(1.5)
  })
  it("비우면 null 허용 칸은 null(원래대로), 아니면 무시(undefined)", () => {
    expect(parseSliderInput("", size)).toBeNull()
    expect(parseSliderInput("  ", size)).toBeNull()
    expect(parseSliderInput("", stroke)).toBeUndefined()
  })
  it("숫자가 아니면 무시(undefined)", () => {
    expect(parseSliderInput("abc", size)).toBeUndefined()
    expect(parseSliderInput("1e999", size)).toBeUndefined()
  })
})
