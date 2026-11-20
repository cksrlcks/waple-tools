import type { Field } from "./schema-to-fields"

/** 화면에 그릴 필드: `x-hidden` 필드를 뺀다 (기본값은 옵션에 그대로 남는다) */
export function visibleFields(fields: Field[]): Field[] {
  return fields.filter((f) => !("hidden" in f && f.hidden))
}
