import { describe, expect, it } from "vitest"

import { noteText } from "./note-text"

describe("noteText", () => {
  it("마크다운 기호·빈 줄을 빼고 목록은 •로", () => {
    expect(noteText("\n**이미지 최적화**\n\n### 새 기능\n- a")).toBe(
      "이미지 최적화\n새 기능\n• a",
    )
    expect(noteText(undefined)).toBeUndefined()
  })
})
