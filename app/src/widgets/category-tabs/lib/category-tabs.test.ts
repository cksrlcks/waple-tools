import { describe, expect, it } from "vitest"

import { categoryTabs } from "./category-tabs"

describe("categoryTabs", () => {
  it("주 탭 4개는 순서 고정 (도구가 없어도 보인다)", () => {
    expect(categoryTabs(false)).toEqual([
      { key: "image", label: "이미지" },
      { key: "pdf", label: "PDF" },
      { key: "web", label: "웹폰트" },
      { key: "misc", label: "기타" },
    ])
  })
  it("개발용 탭은 개발 빌드에서만 맨 끝", () => {
    const tabs = categoryTabs(true)
    expect(tabs.map((t) => t.key)).toEqual([
      "image",
      "pdf",
      "web",
      "misc",
      "dev",
    ])
    expect(tabs[4]).toEqual({ key: "dev", label: "개발용" })
  })
})
