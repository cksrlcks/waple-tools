import { describe, expect, it } from "vitest"

import { toClassSnippet, toDataUri, toJsx } from "./copy-formats"

describe("copy-formats", () => {
  it("toJsx", () => {
    expect(toJsx('<path stroke-width="2" stroke-linecap="round" data-x="1"/>')).toBe(
      '<path strokeWidth="2" strokeLinecap="round" data-x="1"/>',
    )
    expect(toJsx('<path aria-hidden="true"/>')).toBe('<path aria-hidden="true"/>')
  })
  it("toDataUri", () => {
    expect(toDataUri("<svg/>")).toBe('url("data:image/svg+xml,%3Csvg%2F%3E")')
  })
  it("toClassSnippet", () => {
    expect(toClassSnippet("tabler", "home")).toBe('<i class="ti ti-home"></i>')
    expect(toClassSnippet("ri", "home-line")).toBe('<i class="ri-home-line"></i>')
    expect(toClassSnippet("heroicons", "home")).toBeNull()
  })
})
