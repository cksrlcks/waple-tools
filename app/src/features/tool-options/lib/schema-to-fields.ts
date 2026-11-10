/** 도구 옵션 JSON Schema(schemars)를 폼 필드 목록으로 바꾼다. 그릴 수 없는 타입은 unsupported. */

/** 다른 필드 값 조건: { 필드: [허용 값…] } — 모든 키가 맞으면 참 */
export type Condition = Record<string, unknown[]>

type Base = {
  key: string
  label: string
  description?: string
  /** 활성 조건 (`x-enabled-when`). 배열 중 하나라도 참이면 활성. 없으면 항상 활성 */
  enabledWhen?: Condition[]
  /** 묶음 이름 (`x-group`) */
  group?: string
  /** 묶음 제목 자리에 그리는 스위치 (`x-group-toggle`) */
  groupToggle?: boolean
}

export type Field =
  | (Base & { kind: "checkbox"; default: boolean })
  | (Base & {
      kind: "slider"
      min: number
      max: number
      step: number
      default: number
    })
  | (Base & {
      kind: "number"
      min?: number
      max?: number
      integer: boolean
      /** 스키마가 null을 허용하는가 (Option<T>) */
      nullable: boolean
      default: number | null
    })
  | (Base & {
      kind: "select"
      options: { value: string; label: string }[]
      default: string
    })
  | (Base & { kind: "color"; default: string })
  | (Base & { kind: "text"; default: string })
  | (Pick<Base, "key" | "label" | "enabledWhen" | "group"> & {
      kind: "unsupported"
    })

type Json = Record<string, unknown>

const isObj = (v: unknown): v is Json =>
  typeof v === "object" && v !== null && !Array.isArray(v)
const num = (v: unknown) => (typeof v === "number" ? v : undefined)

/** `type`이 "integer" 또는 ["integer", "null"] 같은 배열일 때 null을 뺀 타입 */
function baseType(p: Json): string | undefined {
  const t = p.type
  if (typeof t === "string") return t
  if (Array.isArray(t)) return t.find((x) => x !== "null") as string | undefined
  return undefined
}

/** `$ref: "#/$defs/X"`를 한 단계 풀어 원래 속성과 합친다 */
function resolve(p: Json, root: Json): Json {
  const ref = p.$ref
  if (typeof ref !== "string") return p
  const name = ref.replace(/^#\/(\$defs|definitions)\//, "")
  const defs = (
    isObj(root.$defs)
      ? root.$defs
      : isObj(root.definitions)
        ? root.definitions
        : {}
  ) as Json
  const target = defs[name]
  return isObj(target) ? { ...target, ...p } : p
}

function options(p: Json): { value: string; label: string }[] | null {
  if (Array.isArray(p.enum) && p.enum.every((v) => typeof v === "string")) {
    return (p.enum as string[]).map((v) => ({ value: v, label: v }))
  }
  if (
    Array.isArray(p.oneOf) &&
    p.oneOf.every((o) => isObj(o) && typeof o.const === "string")
  ) {
    return (p.oneOf as Json[]).map((o) => ({
      value: o.const as string,
      label:
        typeof o.description === "string" ? o.description : (o.const as string),
    }))
  }
  return null
}

function conditions(v: unknown): Condition[] | undefined {
  const list = Array.isArray(v) ? v : isObj(v) ? [v] : []
  const valid = list.filter(isObj) as Condition[]
  return valid.length ? valid : undefined
}

/** 확장 키(x-group·x-group-toggle·x-enabled-when). 없는 키는 넣지 않는다 */
function extensions(
  p: Json,
): Pick<Base, "enabledWhen" | "group" | "groupToggle"> {
  const enabledWhen = conditions(p["x-enabled-when"])
  return {
    ...(enabledWhen ? { enabledWhen } : {}),
    ...(typeof p["x-group"] === "string" ? { group: p["x-group"] } : {}),
    ...(p["x-group-toggle"] === true ? { groupToggle: true } : {}),
  }
}

function base(key: string, p: Json): Base {
  const title = typeof p.title === "string" ? p.title : undefined
  const desc = typeof p.description === "string" ? p.description : undefined
  // 제목이 없으면 Rust 주석 전체가 description으로 온다: 첫 문단은 라벨, 빈 줄 뒤는 물음표 설명
  const [head, ...rest] = (desc ?? "").split(/\r?\n\s*\r?\n/)
  const help = rest.join("\n\n").trim()
  const label = title
    ? { key, label: title, ...(desc ? { description: desc } : {}) }
    : {
        key,
        label: head?.trim() || key,
        ...(help ? { description: help } : {}),
      }
  return { ...label, ...extensions(p) }
}

function toField(key: string, raw: Json, root: Json): Field {
  const p = resolve(raw, root)
  const b = base(key, p)
  const opts = options(p)
  if (opts) {
    const d = typeof p.default === "string" ? p.default : (opts[0]?.value ?? "")
    return { ...b, kind: "select", options: opts, default: d }
  }
  const t = baseType(p)
  if (t === "boolean")
    return { ...b, kind: "checkbox", default: p.default === true }
  if (t === "integer" || t === "number") {
    const integer = t === "integer"
    const min = num(p.minimum)
    const max = num(p.maximum)
    if (min !== undefined && max !== undefined) {
      return {
        ...b,
        kind: "slider",
        min,
        max,
        step: integer ? 1 : 0.1,
        default: num(p.default) ?? min,
      }
    }
    return {
      ...b,
      kind: "number",
      ...(min !== undefined ? { min } : {}),
      ...(max !== undefined ? { max } : {}),
      integer,
      nullable: Array.isArray(p.type) && p.type.includes("null"),
      default: num(p.default) ?? null,
    }
  }
  if (t === "string") {
    const d = typeof p.default === "string" ? p.default : ""
    return p.format === "color"
      ? { ...b, kind: "color", default: d || "#ffffff" }
      : { ...b, kind: "text", default: d }
  }
  const { enabledWhen, group } = b
  return {
    kind: "unsupported",
    key,
    label: b.label,
    ...(enabledWhen ? { enabledWhen } : {}),
    ...(group ? { group } : {}),
  }
}

export function schemaToFields(schema: unknown): Field[] {
  if (!isObj(schema) || !isObj(schema.properties)) return []
  return Object.entries(schema.properties)
    .filter((entry): entry is [string, Json] => isObj(entry[1]))
    .map(([key, p]) => toField(key, p, schema))
}
