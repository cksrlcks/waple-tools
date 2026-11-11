export type PreviewMode = "compare" | "result" | "none"

/**
 * 미리보기 방식. 스키마 루트에서 본다.
 * - `x-preview: "result"`: 늘 결과만 (자르기·회전처럼 모양이 바뀌는 도구)
 * - `x-preview: "none"`: 미리보기 없음 (결과가 이미지가 아닌 도구, 예: PDF)
 * - `x-preview-result-when: { 옵션: [값…] }`: 지금 옵션이 모두 맞으면 결과만 (예: 크기 바꾸기를 켰을 때)
 * 그 밖에는 원본과 비교
 */
export function previewMode(
  schema: unknown,
  values: Record<string, unknown> = {},
): PreviewMode {
  if (typeof schema !== "object" || schema === null) return "compare"
  const s = schema as Record<string, unknown>
  if (s["x-preview"] === "result") return "result"
  if (s["x-preview"] === "none") return "none"
  const when = s["x-preview-result-when"]
  if (typeof when === "object" && when !== null) {
    const hit = Object.entries(when as Record<string, unknown>).every(
      ([k, allowed]) => Array.isArray(allowed) && allowed.includes(values[k]),
    )
    if (hit) return "result"
  }
  return "compare"
}
