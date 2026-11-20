/**
 * 슬라이더 옆 입력칸 글 → 옵션 값. 범위 안으로 맞추고 단위(step)에 맞춰 반올림한다.
 * 비우면 null 허용 칸은 null(원래대로), 그 밖에 읽을 수 없으면 undefined(입력 무시).
 */
export function parseSliderInput(
  text: string,
  field: { min: number; max: number; step: number; nullable?: boolean },
): number | null | undefined {
  const t = text.trim()
  if (t === "") return field.nullable ? null : undefined
  const n = Number(t.replace(",", "."))
  if (!Number.isFinite(n)) return undefined
  const snapped =
    field.min + Math.round((n - field.min) / field.step) * field.step
  // 0.1 같은 단위의 부동소수 꼬리 지우기
  return Math.min(field.max, Math.max(field.min, Number(snapped.toFixed(6))))
}
