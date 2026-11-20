import { describe, expect, it } from "vitest"

import { applyOption } from "./apply-option"

describe("applyOption", () => {
  it("key를 바꾸고 resets는 기본값으로", () => {
    expect(
      applyOption(
        { set: "a", selected: "home", limit: 800 },
        "set",
        "b",
        ["selected", "limit"],
        { selected: "", limit: 400 },
      ),
    ).toEqual({ set: "b", selected: "", limit: 400 })
  })
  it("기본값이 없으면 지운다", () => {
    expect(applyOption({ a: 1, b: 2 }, "a", 3, ["b"], {})).toEqual({ a: 3 })
  })
  it("원본은 바뀌지 않는다", () => {
    const o = { a: 1 }
    expect(applyOption(o, "a", 2, undefined, {})).toEqual({ a: 2 })
    expect(o).toEqual({ a: 1 })
  })
})
