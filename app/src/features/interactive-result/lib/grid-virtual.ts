/** 고정 크기 카드 격자의 가상 스크롤 계산 (보이는 줄만 그린다) */

/** 폭에 들어가는 열 수: 카드 최소 폭 + 간격 단위, 최소 1 */
export function columnCount(width: number, cardMin: number, gap: number) {
  return Math.max(1, Math.floor((width + gap) / (cardMin + gap)))
}

/** 그릴 줄 범위 `[start, end)`: 보이는 줄 ± `overscan` */
export function visibleRange({
  scrollTop,
  viewport,
  rowHeight,
  rows,
  overscan,
}: {
  scrollTop: number
  viewport: number
  rowHeight: number
  rows: number
  overscan: number
}) {
  const first = Math.floor(scrollTop / rowHeight)
  const last = Math.ceil((scrollTop + viewport) / rowHeight)
  return {
    start: Math.min(rows, Math.max(0, first - overscan)),
    end: Math.min(rows, last + overscan),
  }
}

/** `row` 줄이 아래 `inset`(덮는 패널) 위에 보이게 할 scrollTop. 이미 보이면 그대로 */
export function revealScrollTop({
  scrollTop,
  viewport,
  inset,
  rowHeight,
  row,
}: {
  scrollTop: number
  viewport: number
  inset: number
  rowHeight: number
  row: number
}) {
  const top = row * rowHeight
  const bottom = top + rowHeight
  if (top < scrollTop) return top
  const visibleEnd = scrollTop + viewport - inset
  return bottom > visibleEnd ? bottom - (viewport - inset) : scrollTop
}
