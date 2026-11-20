import { describe, expect, it } from "vitest"

import { scopeIds } from "./scope-ids"

describe("scopeIds", () => {
  it('id와 url(#…)·href="#…" 참조에 접두어를 붙인다', () => {
    expect(scopeIds('<mask id="a"/><path mask="url(#a)"/>', "c1")).toBe(
      '<mask id="c1-a"/><path mask="url(#c1-a)"/>',
    )
    expect(scopeIds('<g id="b"/><use href="#b" xlink:href="#b"/>', "p")).toBe(
      '<g id="p-b"/><use href="#p-b" xlink:href="#p-b"/>',
    )
  })

  it("id가 없으면 그대로", () => {
    const svg = '<path d="M0 0h24" fill="currentColor"/>'
    expect(scopeIds(svg, "c1")).toBe(svg)
  })
})
