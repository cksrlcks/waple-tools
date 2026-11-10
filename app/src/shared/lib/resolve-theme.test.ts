import { describe, expect, it } from "vitest"

import { resolveTheme } from "./resolve-theme"

describe("resolveTheme", () => {
  it("system은 OS 설정을 따른다", () => {
    expect(resolveTheme("system", true)).toBe("dark")
    expect(resolveTheme("system", false)).toBe("light")
  })
  it("직접 고른 값이 OS 설정보다 우선", () => {
    expect(resolveTheme("light", true)).toBe("light")
    expect(resolveTheme("dark", false)).toBe("dark")
  })
})
