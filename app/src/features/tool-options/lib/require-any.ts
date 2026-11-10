/**
 * 옵션 때문에 실행할 수 없는 이유, 없으면 null. 스키마 루트에서 두 가지를 본다.
 * - `x-require-any: [필드…]` — 하나도 true가 아니면 막는다 (예: 이미지 최적화의 단계 스위치)
 * - `x-require-change: { 필드: 그대로일 때 값 }` — 모두 그대로면 막는다 (예: 자르기·회전·여백에서 아무것도 안 고름)
 * 문구는 `x-require-message`가 있으면 그것을 쓴다.
 */
export function requireAnyMissing(
  schema: unknown,
  values: Record<string, unknown>,
): string | null {
  if (typeof schema !== "object" || schema === null) return null
  const s = schema as Record<string, unknown>
  const message =
    typeof s["x-require-message"] === "string"
      ? s["x-require-message"]
      : "적용할 단계를 하나 이상 켜세요."

  const any = s["x-require-any"]
  if (Array.isArray(any) && any.length > 0) {
    if (!any.some((k) => values[String(k)] === true)) return message
  }

  const change = s["x-require-change"]
  if (typeof change === "object" && change !== null) {
    const idle = Object.entries(change as Record<string, unknown>)
    if (idle.length > 0 && idle.every(([k, v]) => values[k] === v))
      return message
  }
  return null
}
