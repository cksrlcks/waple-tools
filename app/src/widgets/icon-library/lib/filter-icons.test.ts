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

  it('"" 키가 없는 세트(ri)는 접미어 없는 아이콘을 모든 스타일에 보여 준다', () => {
    const ri = ["bold", "home-line", "home-fill"].map((name) => ({
      name,
      body: "",
    }))
    const riStyles = { line: "선", fill: "채움" }
    const names = (style: string) =>
      filterIcons(ri, { query: "", style, styles: riStyles }).map((i) => i.name)
    expect(names("line")).toEqual(["bold", "home-line"])
    expect(names("fill")).toEqual(["bold", "home-fill"])
  })

  it("스타일 키가 하나뿐이어도 접미어 없는 아이콘이 사라지지 않는다", () => {
    const one = ["bold", "home-outline"].map((name) => ({ name, body: "" }))
    const oneStyles = { outline: "외곽선" }
    expect(
      filterIcons(one, { query: "", style: "outline", styles: oneStyles }),
    ).toHaveLength(2)
    expect(
      filterIcons(one, { query: "", style: null, styles: oneStyles }),
    ).toHaveLength(2)
  })
})
