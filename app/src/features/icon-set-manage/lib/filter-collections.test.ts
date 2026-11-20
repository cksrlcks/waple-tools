import { describe, expect, it } from "vitest"

import type { Collection } from "@/entities/icon-set"

import { filterCollections } from "./filter-collections"

const c = (prefix: string, name: string) => ({ prefix, name }) as Collection
const list = [c("tabler", "Tabler Icons"), c("ri", "Remix Icon")]

describe("filterCollections", () => {
  it("이름을 대소문자 무시하고 찾는다", () => {
    expect(filterCollections(list, "REMIX").map((x) => x.prefix)).toEqual([
      "ri",
    ])
  })
  it("prefix로도 찾는다", () => {
    expect(filterCollections(list, "Tab").map((x) => x.prefix)).toEqual([
      "tabler",
    ])
  })
  it("빈 검색어는 전부", () => {
    expect(filterCollections(list, "  ")).toEqual(list)
  })
})
