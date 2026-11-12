import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"

import { editorMarks } from "./editor-marks"

const hl: Block = {
  type: "highlight",
  spans: [{ text: "a" }, { text: "b", mark: 0 }],
}
const num: Block = { type: "number", label: "일치 수", value: 1, unit: null }

describe("editorMarks", () => {
  it("글이 같은 편집기에 강조를 옮기고 목록에서 뺀다", () => {
    const r = editorMarks([num, hl], { text: "ab" })
    expect(r.marks.text).toEqual(hl.type === "highlight" ? hl.spans : [])
    expect(r.rest).toEqual([num])
  })
  it("글이 다르면(결과가 옛 글) 그대로 둔다", () => {
    const r = editorMarks([num, hl], { text: "abc" })
    expect(r.marks).toEqual({})
    expect(r.rest).toEqual([num, hl])
  })
})
