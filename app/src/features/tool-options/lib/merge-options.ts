import type { Field } from "./schema-to-fields"

const clamp = (v: number, min?: number, max?: number) =>
  Math.min(max ?? Infinity, Math.max(min ?? -Infinity, v))

/**
 * 필드 기본값에서 시작해 저장된 값 중 쓸 수 있는 것만 덮어쓴다.
 * 예전 버전에서 저장된 값(없어진 키, 범위 밖, 타입 다름)이 와도 항상 올바른 옵션을 만든다.
 */
export function mergeOptions(
  fields: Field[],
  saved: unknown,
): Record<string, unknown> {
  const s =
    typeof saved === "object" && saved !== null
      ? (saved as Record<string, unknown>)
      : {}
  const out: Record<string, unknown> = {}
  for (const f of fields) {
    if (f.kind === "unsupported") continue
    const v = s[f.key]
    switch (f.kind) {
      case "checkbox":
        out[f.key] = typeof v === "boolean" ? v : f.default
        break
      case "slider":
        out[f.key] =
          typeof v === "number"
            ? round(clamp(v, f.min, f.max), f.step === 1)
            : f.default
        break
      case "number":
        out[f.key] =
          typeof v === "number"
            ? round(clamp(v, f.min, f.max), f.integer)
            : v === null && f.nullable
              ? null
              : (f.default ?? (f.nullable ? null : (f.min ?? 0)))
        break
      case "select":
        out[f.key] =
          typeof v === "string" && f.options.some((o) => o.value === v)
            ? v
            : f.default
        break
      case "color":
      case "text":
      case "file":
      case "multiline":
      case "regex":
        out[f.key] = typeof v === "string" ? v : f.default
        break
    }
  }
  return out
}

function round(v: number, integer: boolean) {
  return integer ? Math.round(v) : v
}
