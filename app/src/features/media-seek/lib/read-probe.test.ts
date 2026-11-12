import { describe, expect, it } from "vitest"

import { readProbe } from "./read-probe"

describe("readProbe", () => {
  it("길이·fps 숫자 블록을 읽는다", () => {
    expect(
      readProbe([
        { type: "number", label: "duration", value: 4, unit: "s" },
        { type: "number", label: "fps", value: 15, unit: null },
      ]),
    ).toEqual({ duration: 4, fps: 15 })
  })
  it("빠졌거나 0이면 null", () => {
    expect(
      readProbe([{ type: "number", label: "duration", value: 4, unit: "s" }]),
    ).toBeNull()
    expect(
      readProbe([
        { type: "number", label: "duration", value: 0, unit: "s" },
        { type: "number", label: "fps", value: 15, unit: null },
      ]),
    ).toBeNull()
    expect(readProbe([{ type: "text", text: "fps" }])).toBeNull()
  })
})
