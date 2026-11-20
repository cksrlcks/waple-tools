import { describe, expect, it } from "vitest"

import { filterByTab } from "./filter-by-tab"

const items = [
  { id: "a", title: "A", meta: "" },
  { id: "b", title: "B", meta: "" },
  { id: "c", title: "C", meta: "" },
]

describe("filterByTab", () => {
  it("설치됨은 받은 항목만, 순서 그대로", () => {
    expect(
      filterByTab(items, new Set(["c", "a", "zz"]), "installed").map(
        (i) => i.id,
      ),
    ).toEqual(["a", "c"])
  })
  it("전체는 그대로", () => {
    expect(filterByTab(items, new Set(["a"]), "all")).toEqual(items)
  })
  it("받은 게 없으면 설치됨은 빈 목록", () => {
    expect(filterByTab(items, new Set(), "installed")).toEqual([])
  })
})
