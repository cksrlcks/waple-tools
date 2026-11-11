import { describe, expect, it } from "vitest"

import type { FileResult, JobReport } from "@/entities/tool"

import { commonDir, summarizeReport } from "./summarize-report"

const done = (
  input: string,
  out: string,
  before: number,
  after: number,
): FileResult => ({
  status: "done",
  input,
  outputs: [out],
  backup: null,
  sizeBefore: before,
  sizeAfter: after,
})

describe("summarizeReport", () => {
  it("상태별로 세고 용량은 성공한 것만 합친다", () => {
    const report: JobReport = {
      jobId: "j",
      files: [
        done("C:\\a\\x.png", "C:\\a\\x.webp", 1000, 400),
        done("C:\\a\\y.png", "C:\\a\\y.webp", 1000, 600),
        { status: "failed", input: "C:\\a\\z.png", message: "오류" },
        { status: "skipped", input: "C:\\a\\w.png", reason: "용량 증가" },
        { status: "cancelled", input: "C:\\a\\v.png" },
      ],
      skipped: [
        { path: "C:\\a\\b.txt", reason: "지원하지 않는 형식" },
        { path: "C:\\a\\c.txt", reason: "지원하지 않는 형식" },
      ],
    }
    expect(summarizeReport(report)).toEqual({
      done: 2,
      failed: 1,
      skipped: 3,
      cancelled: 1,
      sizeBefore: 2000,
      sizeAfter: 1000,
      change: "2.0 KB → 1000 B (-50%)",
      openDir: "C:\\a",
    })
  })
  it("성공이 없으면 용량 변화·폴더 없음", () => {
    const s = summarizeReport({
      jobId: "j",
      files: [{ status: "cancelled", input: "a" }],
      skipped: [],
    })
    expect(s.change).toBeNull()
    expect(s.openDir).toBeNull()
  })
  it("슬래시 경로도 부모 폴더를 찾는다", () => {
    const s = summarizeReport({
      jobId: "j",
      files: [done("/a/x", "/out/sub/x", 1, 1)],
      skipped: [],
    })
    expect(s.openDir).toBe("/out/sub")
  })
})

describe("결과 세트 (Generator)", () => {
  it("세트 폴더를 열고 용량 변화는 보이지 않는다", () => {
    const report: JobReport = {
      jobId: "j",
      files: [
        {
          status: "done",
          input: "C:\\a\\logo.png",
          outputs: [
            "C:\\a\\logo-icons\\favicon\\favicon.ico",
            "C:\\a\\logo-icons\\ios\\AppIcon.appiconset\\Icon-1024.png",
          ],
          backup: null,
          sizeBefore: 10,
          sizeAfter: 5000,
        },
      ],
      skipped: [],
    }
    const s = summarizeReport(report, true)
    expect(s.openDir).toBe("C:\\a\\logo-icons")
    expect(s.change).toBeNull()
  })

  it("공통 폴더", () => {
    expect(commonDir([])).toBeNull()
    expect(commonDir(["/a/b/c.txt"])).toBe("/a/b")
    expect(commonDir(["/a/b/c.txt", "/a/b/d/e.txt"])).toBe("/a/b")
  })

  it("세트 도구는 용량 변화를 숨기고, showSize면 보여 준다", () => {
    const report: JobReport = {
      jobId: "j",
      files: [done("C:/a/x.ttf", "C:/a/x-subset/x.woff2", 1000, 400)],
      skipped: [],
    }
    expect(summarizeReport(report, true).change).toBeNull()
    expect(summarizeReport(report, true, true).change).not.toBeNull()
  })
})
