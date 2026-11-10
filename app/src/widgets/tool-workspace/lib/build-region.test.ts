import { describe, expect, it, vi } from "vitest"

import { buildRegion } from "./build-region"

const keys = {
  x: "cropX",
  y: "cropY",
  w: "width",
  h: "height",
  ratio: "ratio",
  angle: "angle",
  flipH: "flipH",
  flipV: "flipV",
  when: { crop: ["manual"] },
}
const values = {
  crop: "manual",
  cropX: 10,
  cropY: 20,
  width: 300,
  height: 200,
  ratio: "r3x2",
  angle: "r90",
  flipH: true,
  flipV: false,
}

describe("buildRegion", () => {
  it("조건이 안 맞거나 선언이 없으면 null", () => {
    expect(buildRegion(keys, { ...values, crop: "ratio" }, vi.fn())).toBeNull()
    expect(buildRegion(null, values, vi.fn())).toBeNull()
  })
  it("옵션 값을 비율·회전으로 읽는다", () => {
    const r = buildRegion(keys, values, vi.fn())!
    expect(r.ratio).toBeCloseTo(1.5)
    expect([r.angle, r.flipH, r.flipV]).toEqual([90, true, false])
  })
  it("영역을 바꾸면 해당 옵션 키로 돌려준다", () => {
    const set = vi.fn()
    buildRegion(keys, values, set)!.onChange({ x: 1, y: 2, w: 3, h: 4 })
    expect(set).toHaveBeenCalledWith({
      cropX: 1,
      cropY: 2,
      width: 3,
      height: 4,
    })
  })
})
