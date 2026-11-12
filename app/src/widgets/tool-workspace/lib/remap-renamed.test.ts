import { describe, expect, it } from "vitest"

import type { JobReport } from "@/entities/tool"

import { remapRenamed } from "./remap-renamed"

const report: JobReport = {
  jobId: "j",
  skipped: [],
  files: [
    {
      status: "done",
      input: "C:\\d\\a.txt",
      outputs: ["C:\\d\\b.txt"],
      backup: null,
      sizeBefore: 1,
      sizeAfter: 1,
    },
    { status: "failed", input: "C:\\d\\c.txt", message: "겹침" },
  ],
}

describe("remapRenamed", () => {
  it("follows renamed files and leaves the rest", () => {
    expect(
      remapRenamed(["C:\\d\\a.txt", "C:\\d\\c.txt", "C:\\d"], report),
    ).toEqual(["C:\\d\\b.txt", "C:\\d\\c.txt", "C:\\d"])
  })

  it("goes back only for restored files", () => {
    expect(
      remapRenamed(["C:\\d\\b.txt"], report, [
        { status: "restored", path: "C:\\d\\a.txt" },
      ]),
    ).toEqual(["C:\\d\\a.txt"])
    expect(
      remapRenamed(["C:\\d\\b.txt"], report, [
        { status: "skipped", path: "C:\\d\\a.txt", reason: "수정됨" },
      ]),
    ).toEqual(["C:\\d\\b.txt"])
  })
})
