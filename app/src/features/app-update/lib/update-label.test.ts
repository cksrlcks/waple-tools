import { describe, expect, it } from "vitest"

import { percentOf, updateLabel } from "./update-label"

describe("updateLabel", () => {
  it("상태마다 버튼 글자", () => {
    expect(updateLabel({ kind: "idle" })).toBe("업데이트 확인")
    expect(updateLabel({ kind: "latest" })).toBe("최신입니다.")
    expect(updateLabel({ kind: "available", version: "0.2.0" })).toBe(
      "v0.2.0 설치",
    )
    expect(updateLabel({ kind: "installing", percent: 42 })).toBe(
      "설치 중… 42%",
    )
    expect(updateLabel({ kind: "installing", percent: null })).toBe("설치 중…")
  })
})

describe("percentOf", () => {
  it("전체 크기를 알면 %, 모르면 null", () => {
    expect(percentOf(50, 200)).toBe(25)
    expect(percentOf(300, 200)).toBe(100)
    expect(percentOf(10, undefined)).toBeNull()
    expect(percentOf(10, 0)).toBeNull()
  })
})
