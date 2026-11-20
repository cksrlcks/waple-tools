import { schemaToFields, type Field } from "./schema-to-fields"

/** `x-run-buttons` 필드 (값마다 버튼, 누를 때만 실행). 없으면 null */
export function runButtonsField(
  schema: unknown,
): Extract<Field, { kind: "select" }> | null {
  for (const f of schemaToFields(schema)) {
    if (f.kind === "select" && f.runButtons) return f
  }
  return null
}
