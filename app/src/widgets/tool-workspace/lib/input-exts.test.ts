import { describe, expect, it } from "vitest"

import type { PathStat } from "@/entities/tool"

import { inputExts } from "./input-exts"

const file = (path: string, isDir = false) =>
  ({ path, isDir, exists: true, size: 0 }) satisfies PathStat

describe("inputExts", () => {
  it("파일 확장자를 소문자로, 중복 없이", () => {
    expect(
      inputExts([file("C:\\a\\x.PNG"), file("C:/a/y.png"), file("b.jpg")], []),
    ).toEqual(["png", "jpg"])
  })
  it("폴더는 도구가 받는 확장자 전부", () => {
    expect(inputExts([file("C:\\photos", true)], ["png", "webp"])).toEqual([
      "png",
      "webp",
    ])
  })
  it("아무것도 없으면 빈 목록", () => {
    expect(inputExts([], ["png"])).toEqual([])
  })
})
