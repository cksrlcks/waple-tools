import { describe, expect, it } from "vitest"

import { mergeOptions } from "./merge-options"
import type { Field } from "./schema-to-fields"

const fields: Field[] = [
  {
    kind: "slider",
    key: "quality",
    label: "품질",
    min: 0,
    max: 100,
    step: 1,
    default: 80,
  },
  { kind: "checkbox", key: "lossless", label: "무손실", default: false },
  {
    kind: "select",
    key: "mode",
    label: "모드",
    options: [
      { value: "quality", label: "품질" },
      { value: "size", label: "용량" },
    ],
    default: "quality",
  },
  {
    kind: "number",
    key: "targetKb",
    label: "목표",
    min: 1,
    integer: true,
    nullable: true,
    default: null,
  },
  {
    kind: "number",
    key: "width",
    label: "너비",
    min: 1,
    integer: true,
    nullable: false,
    default: 800,
  },
  { kind: "unsupported", key: "sizes", label: "너비 목록" },
]

describe("mergeOptions", () => {
  it("저장값이 없으면 기본값 (unsupported는 넣지 않음)", () => {
    expect(mergeOptions(fields, null)).toEqual({
      quality: 80,
      lossless: false,
      mode: "quality",
      targetKb: null,
      width: 800,
    })
  })
  it("범위 밖 숫자는 자르고, 모르는 키는 버리고, 타입이 틀리면 기본값", () => {
    expect(
      mergeOptions(fields, { quality: 150, gone: 1, lossless: "yes" }),
    ).toEqual({
      quality: 100,
      lossless: false,
      mode: "quality",
      targetKb: null,
      width: 800,
    })
  })
  it("select에 없는 값은 기본값", () => {
    expect(mergeOptions(fields, { mode: "zzz" }).mode).toBe("quality")
    expect(mergeOptions(fields, { mode: "size" }).mode).toBe("size")
  })
  it("정수 필드는 반올림, 아래 범위도 자름", () => {
    expect(mergeOptions(fields, { quality: 42.6 }).quality).toBe(43)
    expect(mergeOptions(fields, { targetKb: -5 }).targetKb).toBe(1)
  })
  it("null을 허용하지 않는 숫자에 null이 오면 기본값", () => {
    expect(mergeOptions(fields, { width: null }).width).toBe(800)
  })
  it("nullable 숫자는 null 유지", () => {
    expect(mergeOptions(fields, { targetKb: null }).targetKb).toBe(null)
    expect(mergeOptions(fields, { targetKb: 300 }).targetKb).toBe(300)
  })
})
