import { describe, expect, it } from "vitest"

import type { ToolEntry } from "@/entities/tool"

import { groupTools } from "./group-tools"

const tool = (id: string): ToolEntry => ({
  id,
  name: id,
  category: id.split(".")[0],
  description: "",
  status: "builtin",
  kind: "perFile",
  optionsSchema: {},
  extensions: [],
  version: null,
  latest: null,
  size: null,
})

const tools = [tool("image.a"), tool("image.b"), tool("image.c")]

describe("groupTools (탭 안 목록)", () => {
  it("즐겨찾기를 맨 앞, 나머지는 '도구'", () => {
    const g = groupTools(tools, ["image.c", "image.a"])
    expect(g.map((x) => x.key)).toEqual(["favorites", "all"])
    expect(g[0].label).toBe("즐겨찾기")
    expect(g[0].tools.map((t) => t.id)).toEqual(["image.c", "image.a"])
    expect(g[1].label).toBe("도구")
    expect(g[1].tools.map((t) => t.id)).toEqual(["image.b"])
  })
  it("즐겨찾기가 없으면 '도구' 하나", () => {
    const g = groupTools(tools, [])
    expect(g.map((x) => x.key)).toEqual(["all"])
  })
  it("다른 탭의 즐겨찾기 id는 무시, 빈 그룹 생략", () => {
    const g = groupTools([tool("image.a")], ["dev.copy", "image.a"])
    expect(g.map((x) => x.key)).toEqual(["favorites"])
  })
})
