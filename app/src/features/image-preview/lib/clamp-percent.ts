/** 상자 안 가로 위치 → 0~100 비율 (폭이 0이면 가운데) */
export function clampPercent(x: number, width: number): number {
  if (width <= 0) return 50
  return Math.min(100, Math.max(0, (x / width) * 100))
}
