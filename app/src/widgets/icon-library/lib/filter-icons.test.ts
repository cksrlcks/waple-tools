import { describe, expect, it } from "vitest"

import { filterIcons } from "./filter-icons"

describe("filterIcons", () => {
  const icons = ["home", "home-filled", "arrow-left", "arrow-left-filled"].map(
    (name) => ({ name, body: "" }),
  )
  const styles = { "": "선", filled: "채움" }

  it("스타일과 검색어(모든 단어, 대소문자 무시)로 거른다", () => {
    expect(
      filterIcons(icons, { query: "", style: "", styles }).map((i) => i.name),
    ).toEqual(["home", "arrow-left"])
    expect(
      filterIcons(icons, { query: "LEFT arrow", style: "filled", styles }).map(
        (i) => i.name,
      ),
    ).toEqual(["arrow-left-filled"])
    expect(
      filterIcons(icons, { query: "home", style: null, styles }),
    ).toHaveLength(2)
  })
})
