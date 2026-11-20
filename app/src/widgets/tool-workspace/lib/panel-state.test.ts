import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"
import type { Field } from "@/features/tool-options"

import { changedEntries, panelState } from "./panel-state"

const fields: Field[] = [
  {
    kind: "slider",
    key: "size",
    label: "크기",
    min: 8,
    max: 512,
    step: 1,
    default: null,
    nullable: true,
  },
  {
    kind: "slider",
    key: "stroke",
    label: "두께",
    min: 0.5,
    max: 3,
    step: 0.25,
    default: null,
    nullable: true,
  },
]
const image: Block = {
  type: "image",
  src: "data:image/svg+xml,<svg/>",
  name: "a.svg",
}

describe("panelState", () => {
  it("결과의 optionHints로 힌트·숨김을 모으고 숨길 칸을 뺀다", () => {
    const data: Block[] = [
      image,
      { type: "optionHints", values: { size: 24 }, hidden: ["stroke"] },
    ]
    expect(
      panelState({
        title: "도구",
        fields,
        options: { size: null },
        blocks: [image],
        data,
        fetching: true,
        error: null,
      }),
    ).toEqual({
      title: "도구",
      fields: [fields[0]],
      options: { size: null },
      blocks: [image],
      hints: { size: 24 },
      hidden: ["stroke"],
      fading: true,
      error: null,
    })
  })

  it("결과가 아직 없으면 힌트·숨김 없이 칸 모두", () => {
    const s = panelState({
      title: "도구",
      fields,
      options: {},
      blocks: [],
      data: undefined,
      fetching: false,
      error: "오류",
    })
    expect(s.fields).toEqual(fields)
    expect(s.hints).toEqual({})
    expect(s.hidden).toEqual([])
    expect(s.error).toBe("오류")
  })
})

describe("changedEntries", () => {
  it("값이 바뀐 키만 [키, 값]으로", () => {
    expect(
      changedEntries({ a: 1, b: null, c: "x" }, { a: 1, b: 2, c: "y" }),
    ).toEqual([
      ["b", 2],
      ["c", "y"],
    ])
  })
  it("같으면 빈 목록 (null도 같은 값)", () => {
    expect(changedEntries({ a: null }, { a: null })).toEqual([])
  })
})
