import { describe, expect, it } from "vitest"

import type { ToolEntry } from "@/entities/tool"

import { categoryTabs } from "./category-tabs"

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

describe("categoryTabs", () => {
  const tools = [tool("image.optimize"), tool("dev.copy")]

  it("주 탭 4개는 순서 고정, 도구가 없으면 비활성", () => {
    expect(categoryTabs(tools, false)).toEqual([
      { key: "image", label: "이미지", disabled: false },
      { key: "pdf", label: "PDF", disabled: true },
      { key: "web", label: "웹폰트", disabled: true },
      { key: "misc", label: "기타", disabled: true },
    ])
  })
  it("개발용 탭은 개발 빌드에서만 맨 끝", () => {
    const tabs = categoryTabs(tools, true)
    expect(tabs.map((t) => t.key)).toEqual([
      "image",
      "pdf",
      "web",
      "misc",
      "dev",
    ])
    expect(tabs[4]).toEqual({ key: "dev", label: "개발용", disabled: false })
  })
})
