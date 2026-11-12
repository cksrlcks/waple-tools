/** 초 → `1:05`, 한 시간 넘으면 `1:02:03`. `ms`면 뒤에 밀리초 (`0:12.345`) */
export function formatTime(secs: number, ms = false): string {
  const total = Math.max(0, Math.round(secs * 1000))
  const s = Math.floor(total / 1000)
  const [h, m, sec] = [Math.floor(s / 3600), Math.floor(s / 60) % 60, s % 60]
  const pad = (n: number) => String(n).padStart(2, "0")
  const out = h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`
  return ms ? `${out}.${String(total % 1000).padStart(3, "0")}` : out
}
