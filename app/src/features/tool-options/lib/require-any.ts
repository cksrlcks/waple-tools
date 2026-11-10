/** 스키마 루트 `x-require-any`의 필드가 하나도 켜지지 않았으면 실행을 막는 이유, 아니면 null */
export function requireAnyMissing(
  schema: unknown,
  values: Record<string, unknown>,
): string | null {
  if (typeof schema !== "object" || schema === null) return null
  const keys = (schema as Record<string, unknown>)["x-require-any"]
  if (!Array.isArray(keys) || keys.length === 0) return null
  return keys.some((k) => values[String(k)] === true)
    ? null
    : "적용할 단계를 하나 이상 켜세요."
}
