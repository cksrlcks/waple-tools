import { describe, expect, it } from "vitest"

import { toCsv, toTsv } from "./table-text"

const columns = ["이름", "값"]
const rows = [
  ["a", "1"],
  ['쉼표,와 "따옴표"', "두\n줄"],
  ["탭\t있음", ""],
]

describe("toTsv", () => {
  it("탭으로 칸, 줄바꿈으로 행. 칸 안 탭·줄바꿈은 공백", () => {
    expect(toTsv(columns, rows)).toBe(
      '이름\t값\na\t1\n쉼표,와 "따옴표"\t두 줄\n탭 있음\t',
    )
  })
})

describe("toCsv", () => {
  it("BOM + 쉼표·따옴표·줄바꿈 칸만 따옴표로 감싸고 따옴표는 두 번", () => {
    expect(toCsv(columns, rows)).toBe(
      '﻿이름,값\r\na,1\r\n"쉼표,와 ""따옴표""","두\n줄"\r\n탭\t있음,\r\n',
    )
  })
})
