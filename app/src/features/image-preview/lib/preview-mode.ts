export type PreviewMode = "compare" | "result"

/** 스키마 루트 `x-preview`: "result"면 결과만(자르기·회전처럼 모양이 바뀌는 도구), 그 밖에는 원본과 비교 */
export function previewMode(schema: unknown): PreviewMode {
  if (typeof schema !== "object" || schema === null) return "compare"
  return (schema as Record<string, unknown>)["x-preview"] === "result"
    ? "result"
    : "compare"
}
