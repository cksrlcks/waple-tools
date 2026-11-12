import { describe, expect, it } from "vitest"

import type { RenamePlan, RenameRow } from "@/entities/tool"

import { renameSummary } from "./rename-summary"

const row = (status: RenameRow["status"]): RenameRow =>
  status === "conflict"
    ? { from: "a", to: "b", name: "b", status, reason: "겹침" }
    : { from: "a", to: "b", name: "b", status }

const plan = (...s: RenameRow["status"][]): RenamePlan => ({
  rows: s.map(row),
  skipped: [],
})

describe("renameSummary", () => {
  it("counts statuses and allows a clean plan", () => {
    const s = renameSummary(plan("changed", "unchanged", "changed"), true)
    expect(s).toEqual({
      changed: 2,
      unchanged: 1,
      conflict: 0,
      blockedReason: null,
    })
  })

  it("blocks on conflicts, no changes, or a stale plan", () => {
    expect(renameSummary(plan("changed", "conflict"), true).blockedReason).toBe(
      "충돌하는 이름이 있어 바꿀 수 없습니다.",
    )
    expect(renameSummary(plan("unchanged"), true).blockedReason).toBe(
      "바뀌는 이름이 없습니다.",
    )
    expect(renameSummary(plan("changed"), false).blockedReason).toBe(
      "미리보기를 만드는 중입니다.",
    )
    expect(renameSummary(undefined, true).blockedReason).not.toBeNull()
  })
})
