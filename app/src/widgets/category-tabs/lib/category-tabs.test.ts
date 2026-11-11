import { describe, expect, it } from "vitest"

import { categoryTabs } from "./category-tabs"

describe("categoryTabs", () => {
  it("주 탭 5개는 순서 고정 (도구가 없어도 보인다)", () => {
    expect(categoryTabs(false)).toEqual([
      { key: "image", label: "이미지" },
      { key: "pdf", label: "PDF" },
      { key: "video", label: "영상" },
      { key: "web", label: "웹폰트" },
      { key: "misc", label: "기타" },
    ])
  })
  it("개발용 탭은 개발 빌드에서만 맨 끝", () => {
    const tabs = categoryTabs(true)
    expect(tabs.map((t) => t.key)).toEqual([
      "image",
      "pdf",
      "video",
      "web",
      "misc",
      "dev",
    ])
    expect(tabs[5]).toEqual({ key: "dev", label: "개발용" })
  })
})
