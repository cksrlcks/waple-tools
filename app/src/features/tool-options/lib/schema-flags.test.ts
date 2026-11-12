import { describe, expect, it } from "vitest"

import {
  allowsLarger,
  angleValue,
  checksInput,
  isFilesOnly,
  isInputGrid,
  isKeepOriginalOnly,
  isNoInput,
  isSingleInput,
  ratioValue,
  regionActive,
  regionKeys,
  reportsSize,
  seekKey,
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

describe("seekKey", () => {
  it("문자열일 때만 옵션 키", () => {
    expect(seekKey({ "x-seek": "time" })).toBe("time")
    expect(seekKey({ "x-seek": true })).toBeNull()
    expect(seekKey(null)).toBeNull()
  })
})

describe("isNoInput", () => {
  it('"none"일 때만 입력 파일 없음', () => {
    expect(isNoInput({ "x-input": "none" })).toBe(true)
    expect(isNoInput({ "x-input": true })).toBe(false)
    expect(isNoInput({})).toBe(false)
    expect(isNoInput(null)).toBe(false)
  })
})

describe("allowsLarger", () => {
  it("x-allow-larger: true면 늘 허용", () => {
    expect(allowsLarger({ "x-allow-larger": true })).toBe(true)
    expect(allowsLarger({})).toBe(false)
  })
  it("x-allow-larger-when: 옵션이 맞을 때만", () => {
    const schema = { "x-allow-larger-when": { mode: ["png"] } }
    expect(allowsLarger(schema, { mode: "png" })).toBe(true)
    expect(allowsLarger(schema, { mode: "optimize" })).toBe(false)
    expect(allowsLarger(schema)).toBe(false)
  })
})

describe("reportsSize", () => {
  it("x-report-size: true일 때만", () => {
    expect(reportsSize({ "x-report-size": true })).toBe(true)
    expect(reportsSize({})).toBe(false)
  })
})

describe("isFilesOnly · isKeepOriginalOnly", () => {
  it("true일 때만", () => {
    expect(isFilesOnly({ "x-files-only": true })).toBe(true)
    expect(isFilesOnly({})).toBe(false)
    expect(isKeepOriginalOnly({ "x-keep-original": true })).toBe(true)
    expect(isKeepOriginalOnly({ "x-keep-original": "yes" })).toBe(false)
    expect(isInputGrid({ "x-input-grid": true })).toBe(true)
    expect(isInputGrid({})).toBe(false)
    expect(checksInput({ "x-check-input": true })).toBe(true)
    expect(checksInput(null)).toBe(false)
  })
})

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
