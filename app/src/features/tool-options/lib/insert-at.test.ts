import { describe, expect, it } from "vitest"

import { insertAt } from "./insert-at"

describe("insertAt", () => {
  it("커서 자리에 넣고 커서를 뒤로", () => {
    expect(insertAt("ab", 1, 1, "\\d")).toEqual({ value: "a\\db", caret: 3 })
  })
  it("선택 영역을 바꾼다", () => {
    expect(insertAt("abc", 0, 2, "x")).toEqual({ value: "xc", caret: 1 })
  })
  it("괄호 쌍이면 커서를 괄호 안에", () => {
    expect(insertAt("", 0, 0, "(?:)")).toEqual({ value: "(?:)", caret: 3 })
    expect(insertAt("", 0, 0, "[]")).toEqual({ value: "[]", caret: 1 })
  })
  it("완성된 패턴은 커서를 끝에", () => {
    const p = "\\d{4}-\\d{2}"
    expect(insertAt("", 0, 0, p).caret).toBe(p.length)
    expect(insertAt("", 0, 0, "[a-z]").caret).toBe(5)
  })
})
