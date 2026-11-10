import { describe, expect, it } from "vitest"

import { baseName } from "./base-name"

describe("baseName", () => {
  it("handles both separators", () => {
    expect(baseName("C:\\a\\b.png")).toBe("b.png")
    expect(baseName("/a/b/c.jpg")).toBe("c.jpg")
    expect(baseName("solo")).toBe("solo")
  })
})
