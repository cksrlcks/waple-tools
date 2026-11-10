const UNITS = ["B", "KB", "MB", "GB"] as const

/** 1024 단위, 소수 한 자리 (B는 정수) */
export function formatBytes(n: number): string {
  let value = n
  let unit = 0
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024
    unit++
  }
  return unit === 0 ? `${value} B` : `${value.toFixed(1)} ${UNITS[unit]}`
}

/** "45.2 MB → 12.1 MB (-73%)" */
export function formatChange(before: number, after: number): string {
  const sizes = `${formatBytes(before)} → ${formatBytes(after)}`
  if (before === 0) return sizes
  const pct = Math.round(((after - before) / before) * 100)
  return `${sizes} (${pct > 0 ? "+" : ""}${pct}%)`
}
