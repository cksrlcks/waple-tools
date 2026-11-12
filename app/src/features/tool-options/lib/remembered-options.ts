import type { Field } from "./schema-to-fields"

/** 설정 파일에 마지막 옵션으로 남길 값: 비밀번호는 뺀다 */
export function rememberedOptions(
  fields: Field[],
  opts: Record<string, unknown>,
): Record<string, unknown> {
  const out = { ...opts }
  for (const f of fields) if (f.kind === "password") delete out[f.key]
  return out
}
