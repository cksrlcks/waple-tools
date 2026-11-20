import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"

import { splitStageBlocks } from "./split-stage-blocks"

const resourceListBlock: Block = {
  type: "resourceList",
  items: [{ id: "a", title: "A", meta: "1개 · MIT" }],
  option: "set",
  resets: [],
}
const gridBlock: Block = {
  type: "iconGrid",
  cells: [],
  option: "selected",
  total: 0,
}
const textBlock: Block = { type: "text", text: "안내" }
const imageBlock: Block = { type: "image", src: "data:,", name: "a.png" }
const codeBlock: Block = { type: "code", lang: "svg", code: "<svg/>" }

describe("splitStageBlocks", () => {
  it("목록·상세 배치: 리소스 목록은 왼쪽, 패널 종류는 패널, 나머지는 작업대", () => {
    const blocks = [
      resourceListBlock,
      gridBlock,
      textBlock,
      imageBlock,
      codeBlock,
    ]
    expect(splitStageBlocks(blocks, "list-detail", ["image", "code"])).toEqual({
      list: [resourceListBlock],
      stage: [gridBlock, textBlock],
      panel: [imageBlock, codeBlock],
    })
  })

  it("배치가 없으면 목록 열은 비고 패널 종류만 패널로 (x-result-panel)", () => {
    const code = codeBlock
    expect(splitStageBlocks([code], null, ["code"])).toEqual({
      list: [],
      stage: [],
      panel: [code],
    })
  })

  it("배치가 없으면 리소스 목록도 작업대에", () => {
    expect(splitStageBlocks([resourceListBlock, textBlock], null, [])).toEqual({
      list: [],
      stage: [resourceListBlock, textBlock],
      panel: [],
    })
  })
})
