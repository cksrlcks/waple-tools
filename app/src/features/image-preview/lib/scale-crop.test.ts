import { describe, expect, it } from "vitest"

import { scaleCrop } from "./scale-crop"

describe("scaleCrop", () => {
  it("화면용 크기 기준 영역을 원본 px로 늘린다", () => {
    expect(
      scaleCrop(
        { x: 100, y: 50, width: 200, height: 100 },
        { w: 1600, h: 1200 },
        { w: 4000, h: 3000 },
      ),
    ).toEqual({ x: 250, y: 125, w: 500, h: 250 })
  })
  it("원본 밖으로 나간 부분은 잘라 맞춘다", () => {
    expect(
      scaleCrop(
        { x: -5, y: 290, width: 500, height: 50 },
        { w: 400, h: 300 },
        { w: 400, h: 300 },
      ),
    ).toEqual({ x: 0, y: 290, w: 400, h: 10 })
  })
})
