import type { Field } from "./schema-to-fields"

/** 필드의 활성 조건(`enabledWhen`) 중 하나라도 현재 값과 맞으면 활성. 조건이 없으면 항상 활성 */
export function isEnabled(
  field: Field,
  values: Record<string, unknown>,
): boolean {
  if (!field.enabledWhen) return true
  return field.enabledWhen.some((cond) =>
    Object.entries(cond).every(([key, allowed]) =>
      allowed.includes(values[key]),
    ),
  )
}
