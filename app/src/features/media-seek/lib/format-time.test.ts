import { describe, expect, it } from "vitest"

import { formatTime } from "./format-time"

describe("formatTime", () => {
  it("분:초, 밀리초는 고를 때만", () => {
    expect(formatTime(65)).toBe("1:05")
    expect(formatTime(12.345, true)).toBe("0:12.345")
    expect(formatTime(0, true)).toBe("0:00.000")
  })
  it("한 시간 넘으면 시:분:초", () => {
    expect(formatTime(3723.5, true)).toBe("1:02:03.500")
  })
  it("음수는 0", () => {
    expect(formatTime(-1)).toBe("0:00")
  })
})
