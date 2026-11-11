import { describe, expect, it } from "vitest"

import { moveItem } from "./move-item"

describe("moveItem", () => {
  it("앞뒤로 옮긴다", () => {
    expect(moveItem(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"])
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"])
    expect(moveItem(["a", "b", "c"], 1, 2)).toEqual(["a", "c", "b"])
  })
  it("범위 밖 목표는 양 끝으로", () => {
    expect(moveItem(["a", "b", "c"], 0, -1)).toEqual(["a", "b", "c"])
    expect(moveItem(["a", "b", "c"], 1, 9)).toEqual(["a", "c", "b"])
  })
  it("잘못된 출발점·같은 자리는 그대로", () => {
    const list = ["a", "b"]
    expect(moveItem(list, 5, 0)).toBe(list)
    expect(moveItem(list, 1, 1)).toBe(list)
  })
})
