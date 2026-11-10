import type { Field } from "./schema-to-fields"

/**
 * 필드의 활성 조건(`enabledWhen`) 중 하나라도 현재 값과 맞으면 활성. 조건이 없으면 항상 활성.
 * 값이 목록이면(예: `$inputExt` 넣은 파일 확장자들) 하나라도 허용 값에 있으면 맞는 것으로 본다.
 */
export function isEnabled(
  field: Field,
  values: Record<string, unknown>,
): boolean {
  if (!field.enabledWhen) return true
  return field.enabledWhen.some((cond) =>
    Object.entries(cond).every(([key, allowed]) => {
      const v = values[key]
      return Array.isArray(v)
        ? v.some((x) => allowed.includes(x))
        : allowed.includes(v)
    }),
  )
}
