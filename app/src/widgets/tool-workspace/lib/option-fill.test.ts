import { describe, expect, it } from "vitest"

import type { Block } from "@/entities/tool"
import type { Field } from "@/features/tool-options"

import { fillPatch, fillTargets, hideFilled } from "./option-fill"

const fields: Field[] = [
  {
    kind: "multiline",
    key: "regions",
    label: "영역",
    default: "",
    fillFrom: "영역 글",
  },
  { kind: "multiline", key: "plain", label: "글", default: "" },
  { kind: "text", key: "src", label: "경로", default: "" },
]
const blocks: Block[] = [
  { type: "code", lang: "ALT", code: "a | b" },
  { type: "code", lang: "영역 글", code: "a\nb" },
]

describe("option-fill", () => {
  it("x-fill-from이 있는 여러 줄 옵션만 채울 대상", () => {
    expect(fillTargets(fields)).toEqual([["regions", "영역 글"]])
  })

  it("빈 칸만 채우고, 사용자가 고친 글은 그대로", () => {
    const t = fillTargets(fields)
    expect(fillPatch(t, blocks, { regions: "" })).toEqual({ regions: "a\nb" })
    expect(fillPatch(t, blocks, { regions: "고침" })).toEqual({})
    expect(fillPatch(t, [blocks[0]!], { regions: "" })).toEqual({})
  })

  it("채운 코드 블록은 결과에서 숨긴다", () => {
    expect(hideFilled(blocks, fillTargets(fields))).toEqual([blocks[0]])
  })
})
