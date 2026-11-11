/**
 * 페이지 범위 문자열 ↔ 고른 쪽 집합. 도구의 범위 문법(`1-3,5,8-`, 빈 칸 = 전체)과 같다.
 * 하나도 고르지 않은 상태는 범위 문법에 없으므로 `NO_PAGES`로 적고, 실행 전에 막는다.
 */
export const NO_PAGES = "0"

/** 1부터 시작하는 쪽 번호 집합. 읽을 수 없는 조각은 건너뛴다 */
export function parsePages(text: string, total: number): Set<number> {
  const t = text.trim()
  const all = new Set(Array.from({ length: total }, (_, i) => i + 1))
  if (!t) return all
  const out = new Set<number>()
  if (t === NO_PAGES) return out
  for (const part of t.split(",")) {
    const m = /^\s*(\d*)\s*(-?)\s*(\d*)\s*$/.exec(part)
    if (!m || (!m[1] && !m[3])) continue
    const a = m[1] ? Number(m[1]) : 1
    const b = m[2] ? (m[3] ? Number(m[3]) : total) : a
    for (let p = Math.max(1, a); p <= Math.min(total, b); p++) out.add(p)
  }
  return out
}

/** 집합 → 범위 문자열. 전부면 빈 칸, 하나도 없으면 `NO_PAGES` */
export function formatPages(pages: Set<number>, total: number): string {
  if (pages.size === 0) return NO_PAGES
  const sorted = [...pages]
    .filter((p) => p >= 1 && p <= total)
    .sort((a, b) => a - b)
  if (sorted.length === total) return ""
  const parts: string[] = []
  let start = sorted[0]
  let prev = start
  for (const p of [...sorted.slice(1), Infinity]) {
    if (p === prev + 1) {
      prev = p
      continue
    }
    parts.push(start === prev ? `${start}` : `${start}-${prev}`)
    start = p
    prev = p
  }
  return parts.join(",")
}
