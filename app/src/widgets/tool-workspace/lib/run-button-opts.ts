import { mergeOptions, type Field } from "@/features/tool-options"

/** 실행 버튼을 눌렀을 때 넘길 옵션: 모든 필드의 기본값 + 누른 값 */
export function runButtonOpts(
  fields: Field[],
  key: string,
  value: string,
): Record<string, unknown> {
  return { ...mergeOptions(fields, {}), [key]: value }
}
