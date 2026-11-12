import { mergeOptions, type Field } from "@/features/tool-options"

/**
 * 도구 화면을 처음 열 때의 옵션: 마지막으로 쓴 값을 정리해 쓰되,
 * 메타데이터는 저장된 값이 없으면 설정의 "기본 메타데이터 처리"를 따른다.
 * 묶음 스위치(크기 바꾸기·자르기 등)는 저장된 값과 상관없이 늘 꺼진 채로 시작한다.
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
  for (const f of fields) {
    if ("groupToggle" in f && f.groupToggle) merged[f.key] = false
    // 여러 줄 글은 기억하지 않는다 (수 MB일 수 있다)
    if (f.kind === "multiline") merged[f.key] = f.default
  }
  return merged
}
