import { describe, expect, it } from "vitest"

import {
  angleValue,
  isSingleInput,
  ratioValue,
  regionActive,
  regionKeys,
} from "./schema-flags"

const region = {
  x: "cropX",
  y: "cropY",
  w: "width",
  h: "height",
  ratio: "ratio",
  angle: "angle",
  flipH: "flipH",
  flipV: "flipV",
  when: { crop: ["manual"] },
}

describe("isSingleInput", () => {
  it("true일 때만 한 파일", () => {
    expect(isSingleInput({ "x-single-input": true })).toBe(true)
    expect(isSingleInput({ "x-single-input": "yes" })).toBe(false)
    expect(isSingleInput(null)).toBe(false)
  })
})

describe("regionKeys", () => {
  it("키 네 개가 모두 문자열이면 읽는다", () => {
    expect(regionKeys({ "x-region": region })).toEqual(region)
  })
  it("빠지거나 모양이 다르면 null", () => {
    expect(regionKeys({})).toBeNull()
    expect(regionKeys({ "x-region": { x: "a", y: "b", w: "c" } })).toBeNull()
  })
})

describe("regionActive", () => {
  const keys = regionKeys({ "x-region": region })!
  it("조건이 맞을 때만", () => {
    expect(regionActive(keys, { crop: "manual" })).toBe(true)
    expect(regionActive(keys, { crop: "ratio" })).toBe(false)
  })
})

describe("ratioValue", () => {
  it("r가로x세로를 비로, free는 null", () => {
    expect(ratioValue("r16x9")).toBeCloseTo(16 / 9)
    expect(ratioValue("r1x1")).toBe(1)
    expect(ratioValue("free")).toBeNull()
    expect(ratioValue(undefined)).toBeNull()
  })
})

describe("angleValue", () => {
  it("r90·r180·r270만 각도로", () => {
    expect(angleValue("r90")).toBe(90)
    expect(angleValue("r270")).toBe(270)
    expect(angleValue("none")).toBe(0)
  })
})
