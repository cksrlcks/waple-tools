import { describe, expect, it } from "vitest"

import { recordsText } from "./records-text"

describe("recordsText", () => {
  it("카드마다 이름표·본문·그룹 줄, 카드 사이 빈 줄", () => {
    expect(
      recordsText([
        {
          title: "매치 1",
          meta: "위치 0–5",
          text: "hello",
          fields: [{ label: "그룹 1", value: "he" }],
        },
        { title: "매치 2", text: "x" },
      ]),
    ).toBe("매치 1 · 위치 0–5\nhello\n그룹 1: he\n\n매치 2\nx")
  })
})
