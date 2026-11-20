import { describe, expect, it } from "vitest"

import { showPanel } from "./show-panel"

describe("showPanel", () => {
  it("x-panel-when이 없으면 편집기 배치가 아닌 한 보인다", () => {
    expect(showPanel(null, {}, false)).toBe(true)
    expect(showPanel(null, {}, true)).toBe(false)
  })

  it("x-panel-when 옵션이 비면 숨기고, 값이 있으면 보인다", () => {
    expect(showPanel("set", { set: "" }, false)).toBe(false)
    expect(showPanel("set", {}, false)).toBe(false)
    expect(showPanel("set", { set: "mdi" }, false)).toBe(true)
  })

  it("편집기 배치면 값이 있어도 숨긴다", () => {
    expect(showPanel("set", { set: "mdi" }, true)).toBe(false)
  })
})
