/** 복사용 탭 구분 글. 칸 안의 탭·줄바꿈은 공백으로 (붙여 넣을 때 칸이 밀리지 않게) */
export function toTsv(columns: string[], rows: string[][]): string {
  const cell = (v: string) => v.replace(/[\t\r\n]+/g, " ")
  return [columns, ...rows].map((r) => r.map(cell).join("\t")).join("\n")
}

/** 저장용 CSV (RFC 4180). 엑셀이 한글을 UTF-8로 읽도록 BOM을 붙인다 */
export function toCsv(columns: string[], rows: string[][]): string {
  const cell = (v: string) =>
    /[",\r\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v
  return `﻿${[columns, ...rows].map((r) => r.map(cell).join(",")).join("\r\n")}\r\n`
}
