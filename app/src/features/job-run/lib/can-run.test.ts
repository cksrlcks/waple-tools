import { describe, expect, it } from "vitest"

import { canRun } from "./can-run"

const inPlace = { type: "InPlaceWithBackup" } as const

describe("canRun", () => {
  it("입력이 있고 정책이 완전하면 실행 가능", () => {
    expect(canRun(["a.png"], inPlace, false)).toEqual({ ok: true })
  })
  it("입력이 없으면 안내", () => {
    expect(canRun([], inPlace, false)).toEqual({
      ok: false,
      reason: "처리할 파일을 추가하세요.",
    })
  })
  it("다른 폴더인데 폴더를 고르지 않음", () => {
    expect(canRun(["a"], { type: "OutputFolder", dir: "" }, false)).toEqual({
      ok: false,
      reason: "저장할 폴더를 선택하세요.",
    })
  })
  it("원본 옆인데 접미사가 비어 있음", () => {
    expect(canRun(["a"], { type: "KeepOriginal", suffix: "" }, false)).toEqual({
      ok: false,
      reason: "접미사를 입력하세요.",
    })
  })
  it("실행·되돌리기 중이면 불가", () => {
    expect(canRun(["a"], inPlace, true)).toEqual({
      ok: false,
      reason: "작업 중입니다.",
    })
  })
  it("접미사에 경로·금지 문자가 있으면 불가", () => {
    const backslash = String.fromCharCode(92)
    for (const suffix of [
      "_a/b",
      `_a${backslash}b`,
      "_..",
      "_a:b",
      "_a*",
      "_a?",
      "_<",
      "_|",
      '_"',
    ]) {
      expect(canRun(["a"], { type: "KeepOriginal", suffix }, false)).toEqual({
        ok: false,
        reason: "접미사에 쓸 수 없는 문자가 있습니다.",
      })
    }
    expect(
      canRun(["a"], { type: "KeepOriginal", suffix: "_small-1" }, false),
    ).toEqual({ ok: true })
  })
})
