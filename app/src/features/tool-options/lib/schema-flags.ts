/** 스키마 루트 확장값 읽기: 한 파일만 받는 도구, 미리보기에서 끄는 영역 */

const root = (schema: unknown): Record<string, unknown> =>
  typeof schema === "object" && schema !== null
    ? (schema as Record<string, unknown>)
    : {}

/** `x-input: "none"`이면 입력 파일 없이 옵션만으로 실행한다 (예: QR 코드 생성) */
export function isNoInput(schema: unknown): boolean {
  return root(schema)["x-input"] === "none"
}

/**
 * 편집기 도구의 화면 배치 (`x-layout`). 둘 다 옵션 패널 없이 옵션을 작업대 위 막대에 둔다.
 * `full`: 편집기 아래 결과, `split`: 왼쪽 편집기 · 오른쪽 결과. 없으면 null (옵션 패널)
 */
export function stageLayout(schema: unknown): "full" | "split" | null {
  const v = root(schema)["x-layout"]
  return v === "full" || v === "split" ? v : null
}

/** `x-result-panel: true`면 대화형 결과의 코드 블록을 작업대가 아니라 옵션 패널 아래에 둔다 */
export function resultPanel(schema: unknown): boolean {
  return root(schema)["x-result-panel"] === true
}

/** `x-single-input: true`면 한 번에 파일 하나만 받는다 */
export function isSingleInput(schema: unknown): boolean {
  return root(schema)["x-single-input"] === true
}

/** `x-files-only: true`면 폴더는 받지 않는다 (예: PDF 비밀번호) */
export function isFilesOnly(schema: unknown): boolean {
  return root(schema)["x-files-only"] === true
}

/** `x-keep-original: true`면 저장 위치를 고르지 않고 늘 원본 옆에 접미사를 붙여 저장한다 */
export function isKeepOriginalOnly(schema: unknown): boolean {
  return root(schema)["x-keep-original"] === true
}

/** `x-input-grid: true`면 작업대에 넣은 파일을 목록 대신 카드 격자로 보인다 (순서 바꾸기 없음) */
export function isInputGrid(schema: unknown): boolean {
  return root(schema)["x-input-grid"] === true
}

/**
 * `x-check-input: true`면 파일을 넣을 때 도구를 `{"x-check-input": true}`로 실행해 확인한다.
 * 도구가 오류를 내면 그 파일은 넣지 않는다 (예: 비밀번호 걸기의 이미 암호 PDF)
 */
export function checksInput(schema: unknown): boolean {
  return root(schema)["x-check-input"] === true
}

/**
 * 파일마다 고르는 옵션 (`x-per-file: { 옵션: { 확장자: 기본값 } }`).
 * 이 옵션은 옵션 패널 대신 파일 목록의 각 파일 옆에 나오고, 확장자별 처음 값을 정할 수 있다.
 */
export function perFileFields(
  schema: unknown,
): Record<string, Record<string, string>> {
  const v = root(schema)["x-per-file"]
  if (typeof v !== "object" || v === null) return {}
  return Object.fromEntries(
    Object.entries(v as Record<string, unknown>).map(([k, defaults]) => [
      k,
      typeof defaults === "object" && defaults !== null
        ? (defaults as Record<string, string>)
        : {},
    ]),
  )
}

/**
 * 결과가 원본보다 커도 그대로 두는가 (예: WOFF2 → TTF는 늘 커진다).
 * `x-allow-larger: true`는 늘, `x-allow-larger-when: { 옵션: [값…] }`은 옵션이 모두 맞을 때만 (예: SVG → PNG)
 */
export function allowsLarger(
  schema: unknown,
  values: Record<string, unknown> = {},
): boolean {
  const r = root(schema)
  if (r["x-allow-larger"] === true) return true
  const when = r["x-allow-larger-when"]
  return (
    typeof when === "object" &&
    when !== null &&
    Object.entries(when as Record<string, unknown>).every(
      ([k, allowed]) => Array.isArray(allowed) && allowed.includes(values[k]),
    )
  )
}

/** `x-report-size: true`면 결과 세트를 만드는 도구라도 처리 전후 용량을 보여 준다 (예: 웹폰트 서브셋) */
export function reportsSize(schema: unknown): boolean {
  return root(schema)["x-report-size"] === true
}

/** 작업대에 페이지 썸네일을 보여 주고 고른 쪽을 적을 옵션 키 (`x-page-picker: "pages"`) */
export function pagePickerKey(schema: unknown): string | null {
  const k = root(schema)["x-page-picker"]
  return typeof k === "string" ? k : null
}

/** 작업대 탐색 막대로 정하는 옵션 키 (`x-seek: "time"`, 값은 초) */
export function seekKey(schema: unknown): string | null {
  const k = root(schema)["x-seek"]
  return typeof k === "string" ? k : null
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
