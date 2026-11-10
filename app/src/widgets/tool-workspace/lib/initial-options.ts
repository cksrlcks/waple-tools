import { mergeOptions, type Field } from "@/features/tool-options"

/**
 * 도구 화면을 처음 열 때의 옵션: 마지막으로 쓴 값을 정리해 쓰되,
 * 메타데이터는 저장된 값이 없으면 설정의 "기본 메타데이터 처리"를 따른다.
 */
export function initialOptions(
  fields: Field[],
  saved: unknown,
  keepMetadata: boolean,
): Record<string, unknown> {
  const merged = mergeOptions(fields, saved)
  const hasSaved =
    typeof saved === "object" && saved !== null && "metadata" in saved
  if ("metadata" in merged && !hasSaved) {
    merged.metadata = keepMetadata ? "keep" : "strip"
  }
  return merged
}
