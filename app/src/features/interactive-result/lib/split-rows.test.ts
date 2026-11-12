import { describe, expect, it } from "vitest"

import type { DiffLine } from "@/entities/tool"

import { toSplitRows } from "./split-rows"

const eq = (n: number): DiffLine => ({
  tag: "equal",
  old: n,
  new: n,
  spans: [{ text: "=" }],
})
const del = (n: number): DiffLine => ({ tag: "delete", old: n, spans: [] })
const ins = (n: number): DiffLine => ({ tag: "insert", new: n, spans: [] })

describe("toSplitRows", () => {
  it("같은 줄은 양쪽, delete·insert 묶음은 한 줄씩 짝짓고 짧은 쪽은 비운다", () => {
    const rows = toSplitRows([
      eq(1),
      del(2),
      del(3),
      ins(2),
      eq(4),
      ins(4),
      ins(5),
      del(5),
    ])
    expect(rows).toEqual([
      { left: eq(1), right: eq(1) },
      { left: del(2), right: ins(2) },
      { left: del(3), right: undefined },
      { left: eq(4), right: eq(4) },
      // delete가 앞에 없는 insert 묶음
      { left: undefined, right: ins(4) },
      { left: undefined, right: ins(5) },
      // insert 뒤에 온 delete는 새 묶음
      { left: del(5), right: undefined },
    ])
  })
  it("빈 목록", () => {
    expect(toSplitRows([])).toEqual([])
  })
})
