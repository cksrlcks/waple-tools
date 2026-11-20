import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"

import { mergePanelPart } from "./merge-panel-part"

const text: Block = { type: "text", text: "목록" }
const oldImage: Block = { type: "image", src: "old", name: "" }
const newImage: Block = { type: "image", src: "new", name: "" }
const oldHints: Block = { type: "optionHints", values: { size: 24 } }
const newHints: Block = { type: "optionHints", values: { size: 20 } }

describe("mergePanelPart", () => {
  it("작업대 블록은 전체 실행, 패널 블록·optionHints는 패널 실행에서", () => {
    expect(
      mergePanelPart(
        [text, oldImage, oldHints],
        [newImage, newHints],
        ["image"],
      ),
    ).toEqual([text, newImage, newHints])
  })
  it("패널 실행의 다른 블록(오류 글 등)도 그대로 붙인다", () => {
    const err: Block = { type: "text", text: "색 오류" }
    expect(mergePanelPart([text, oldImage], [err], ["image"])).toEqual([
      text,
      err,
    ])
  })
  it("패널 실행이 아직 없으면 전체 실행 그대로, 전체 실행이 없으면 없음", () => {
    expect(mergePanelPart([text, oldImage], undefined, ["image"])).toEqual([
      text,
      oldImage,
    ])
    expect(mergePanelPart(undefined, [newImage], ["image"])).toBeUndefined()
  })
})
