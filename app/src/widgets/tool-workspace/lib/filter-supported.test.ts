import { describe, expect, it } from "vitest"

import type { PathStat } from "@/entities/tool"

import { filterSupported } from "./filter-supported"

const stat = (path: string, isDir = false) =>
  ({ path, isDir, exists: true, size: 0 }) satisfies PathStat

describe("filterSupported", () => {
  const exts = ["png", "jpg"]
  it("받는 확장자만 남기고 뺀 개수를 센다 (대소문자 무관)", () => {
    const r = filterSupported(
      [
        stat("C:\\a\\x.PNG"),
        stat("C:/a/note.txt"),
        stat("b.jpg"),
        stat("noext"),
      ],
      exts,
    )
    expect(r.ok.map((s) => s.path)).toEqual(["C:\\a\\x.PNG", "b.jpg"])
    expect(r.rejected).toBe(2)
  })
  it("폴더는 그대로 받는다", () => {
    expect(filterSupported([stat("C:\\photos", true)], exts).rejected).toBe(0)
  })
  it("확장자 목록이 비면 모두 받는다", () => {
    expect(filterSupported([stat("a.txt")], []).ok).toHaveLength(1)
  })
})
