import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"

import { optionHints } from "./option-hints"

describe("optionHints", () => {
  it("optionHints 블록의 값을 모은다 (뒤 블록이 이긴다)", () => {
    const blocks: Block[] = [
      { type: "text", text: "a" },
      { type: "optionHints", values: { stroke: 2, size: 24 } },
      { type: "optionHints", values: { stroke: 1.5 } },
    ]
    expect(optionHints(blocks)).toEqual({ stroke: 1.5, size: 24 })
  })
  it("블록이 없으면 빈 값", () => {
    expect(optionHints(undefined)).toEqual({})
    expect(optionHints([{ type: "text", text: "a" }])).toEqual({})
  })
})
