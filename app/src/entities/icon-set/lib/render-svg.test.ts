import { describe, expect, it } from "vitest"

import type { IconItem } from "../model/types"
import { TEST_SET as set } from "./test-set"
import { iconSize, strokeOf, toSvg } from "./render-svg"

const icon: IconItem = { name: "home", body: '<path stroke="currentColor" stroke-width="2" d="M0 0"/>' }

describe("render-svg", () => {
  it("toSvg: 크기·두께 교체", () => {
    expect(toSvg(icon, set, { size: 48, stroke: 1.5, color: "currentColor" })).toBe(
      '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><path stroke="currentColor" stroke-width="1.5" d="M0 0"/></svg>',
    )
  })
  it("toSvg: 색 교체, 비율 폭", () => {
    expect(toSvg(icon, set, { size: 24, stroke: null, color: "#ff0000" })).toContain('stroke="#ff0000" stroke-width="2"')
    expect(toSvg({ ...icon, width: 48, height: 24 }, set, { size: 24, stroke: null, color: "currentColor" })).toContain(
      'width="48" height="24" viewBox="0 0 48 24"',
    )
  })
  it("iconSize·strokeOf", () => {
    expect(iconSize({ ...icon, width: 20, height: 20 }, set)).toEqual({ width: 20, height: 20 })
    expect(iconSize({ ...icon, width: null, height: null }, set)).toEqual({ width: 24, height: 24 })
    expect(strokeOf('<path fill="x"/>')).toBeNull()
    expect(strokeOf(icon.body)).toBe(2)
  })
})
