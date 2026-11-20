const clamp = (v: number, min: number, max: number) =>
  Math.min(max, Math.max(min, v))

/**
 * 슬라이더 손잡이 자리와 값 글. null(원래대로)이면 숫자 힌트(예: 고른 아이콘의 원래 두께)
 * 자리에 손잡이를 두고 "2 · 원래", 힌트가 없으면 맨 왼쪽에 "원래". 힌트는 옵션에 넣지 않는다.
 */
export function sliderDisplay(
  value: unknown,
  field: { min: number; max: number },
  hint: unknown,
): { thumb: number; badge: string } {
  if (value !== null && value !== undefined)
    return { thumb: Number(value), badge: String(value) }
  if (typeof hint === "number" && Number.isFinite(hint))
    return {
      thumb: clamp(hint, field.min, field.max),
      badge: `${hint} · 원래`,
    }
  return { thumb: field.min, badge: "원래" }
}
