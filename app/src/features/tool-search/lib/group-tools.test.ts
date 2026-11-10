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
})

const tools = [tool("image.a"), tool("image.b"), tool("dev.copy")]

describe("groupTools", () => {
  it("즐겨찾기 그룹이 맨 앞, 즐겨찾기 순서 유지", () => {
    const g = groupTools(tools, ["dev.copy", "image.b"])
    expect(g.map((x) => x.key)).toEqual(["favorites", "image", "dev"])
    expect(g[0].label).toBe("즐겨찾기")
    expect(g[0].tools.map((t) => t.id)).toEqual(["dev.copy", "image.b"])
  })
  it("즐겨찾기가 없으면 그룹 생략, 카테고리 순서는 고정", () => {
    const g = groupTools([tool("dev.copy"), tool("image.a")], [])
    expect(g.map((x) => x.key)).toEqual(["image", "dev"])
    expect(g[0].label).toBe("이미지")
  })
  it("빈 카테고리 생략, 목록에 없는 즐겨찾기 id는 무시", () => {
    const g = groupTools([tool("image.a")], ["gone.x"])
    expect(g.map((x) => x.key)).toEqual(["image"])
  })
})
