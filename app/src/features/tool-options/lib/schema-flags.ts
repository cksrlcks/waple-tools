/** 스키마 루트 확장값 읽기: 한 파일만 받는 도구, 미리보기에서 끄는 영역 */

const root = (schema: unknown): Record<string, unknown> =>
  typeof schema === "object" && schema !== null
    ? (schema as Record<string, unknown>)
    : {}

/** `x-single-input: true`면 한 번에 파일 하나만 받는다 */
export function isSingleInput(schema: unknown): boolean {
  return root(schema)["x-single-input"] === true
}

/** 미리보기 위 영역과 연결된 옵션 키 (`x-region`) */
export type RegionKeys = {
  x: string
  y: string
  w: string
  h: string
  /** 비율 고정에 쓰는 옵션 키 (값 예: "r16x9", "free") */
  ratio?: string
  /** 영역은 이 회전·뒤집기를 적용한 뒤의 이미지 기준 */
  angle?: string
  flipH?: string
  flipV?: string
  /** 이 조건일 때만 영역을 편집한다 (`x-enabled-when`과 같은 모양) */
  when?: Record<string, unknown[]>
}

const str = (v: unknown) => (typeof v === "string" ? v : undefined)

export function regionKeys(schema: unknown): RegionKeys | null {
  const r = root(schema)["x-region"]
  if (typeof r !== "object" || r === null) return null
  const o = r as Record<string, unknown>
  const keys = [o.x, o.y, o.w, o.h]
  if (!keys.every((k) => typeof k === "string")) return null
  return {
    x: o.x as string,
    y: o.y as string,
    w: o.w as string,
    h: o.h as string,
    ratio: str(o.ratio),
    angle: str(o.angle),
    flipH: str(o.flipH),
    flipV: str(o.flipV),
    when:
      typeof o.when === "object" && o.when !== null
        ? (o.when as Record<string, unknown[]>)
        : undefined,
  }
}

/** 지금 옵션 값으로 영역 편집이 켜져 있는지 */
export function regionActive(
  keys: RegionKeys,
  values: Record<string, unknown>,
): boolean {
  if (!keys.when) return true
  return Object.entries(keys.when).every(
    ([k, allowed]) => Array.isArray(allowed) && allowed.includes(values[k]),
  )
}

/** 비율 옵션 값("r16x9")을 가로/세로 비로. "free"·알 수 없는 값은 null(고정 안 함) */
export function ratioValue(v: unknown): number | null {
  const m = typeof v === "string" ? /^r(\d+)x(\d+)$/.exec(v) : null
  return m ? Number(m[1]) / Number(m[2]) : null
}

/** 회전 옵션 값("r90")을 시계 방향 각도로. 그 밖은 0 */
export function angleValue(v: unknown): number {
  const m = typeof v === "string" ? /^r(90|180|270)$/.exec(v) : null
  return m ? Number(m[1]) : 0
}
