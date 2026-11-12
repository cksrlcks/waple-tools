import type { JobReport, UndoItem } from "@/entities/tool"

/**
 * 이름을 바꾼 뒤 넣은 경로를 새 이름으로 (되돌린 뒤면 원래 이름으로) 바꾼다.
 * 되돌리기에서 건너뛴 파일(사용자가 고침 등)은 새 이름에 남아 있으므로 그대로 둔다. 폴더 입력은 그대로.
 */
export function remapRenamed(
  paths: string[],
  report: JobReport,
  undoItems?: UndoItem[],
): string[] {
  const done = report.files.flatMap((f) =>
    f.status === "done" && f.outputs[0]
      ? [[f.input, f.outputs[0]] as const]
      : [],
  )
  if (!undoItems) {
    const forward = new Map(done)
    return paths.map((p) => forward.get(p) ?? p)
  }
  const restored = new Set(
    undoItems.flatMap((i) => (i.status === "restored" ? [i.path] : [])),
  )
  const back = new Map(done.map(([from, to]) => [to, from]))
  return paths.map((p) => {
    const from = back.get(p)
    return from && restored.has(from) ? from : p
  })
}
