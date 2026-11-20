import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"

import { hiddenOptions, optionHints } from "./option-hints"

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

describe("hiddenOptions", () => {
  it("optionHints 블록의 hidden을 모은다 (중복 없이)", () => {
    const blocks: Block[] = [
      { type: "optionHints", values: {}, hidden: ["stroke"] },
      { type: "optionHints", values: {}, hidden: ["stroke", "color"] },
    ]
    expect(hiddenOptions(blocks)).toEqual(["stroke", "color"])
  })
  it("없으면 빈 목록", () => {
    expect(hiddenOptions(undefined)).toEqual([])
    expect(hiddenOptions([{ type: "optionHints", values: { a: 1 } }])).toEqual(
      [],
    )
  })
})
