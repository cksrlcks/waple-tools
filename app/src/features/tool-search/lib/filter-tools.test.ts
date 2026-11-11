import { describe, expect, it } from "vitest"

import type { ToolEntry } from "@/entities/tool"

import { filterTools } from "./filter-tools"

const tool = (id: string, name: string, description = ""): ToolEntry => ({
  id,
  name,
  category: id.split(".")[0],
  description,
  status: "notInstalled",
  kind: null,
  optionsSchema: null,
  extensions: [],
  version: null,
  latest: null,
  size: null,
})

const tools = [
  tool("image.webp", "WebP 변환", "이미지를 WebP로"),
  tool("image.resize", "리사이즈"),
]

describe("filterTools", () => {
  it("빈 검색어면 그대로", () => {
    expect(filterTools(tools, "  ")).toEqual(tools)
  })
  it("이름 부분 일치, 대소문자·공백 무시", () => {
    expect(filterTools(tools, "web p").map((t) => t.id)).toEqual(["image.webp"])
    expect(filterTools(tools, "리사").map((t) => t.id)).toEqual([
      "image.resize",
    ])
  })
  it("id와 설명으로도 찾는다", () => {
    expect(filterTools(tools, "image.res").map((t) => t.id)).toEqual([
      "image.resize",
    ])
    expect(filterTools(tools, "이미지를").map((t) => t.id)).toEqual([
      "image.webp",
    ])
  })
})
