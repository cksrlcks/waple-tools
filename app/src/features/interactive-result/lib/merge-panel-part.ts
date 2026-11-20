import type { Block } from "@/entities/tool"

/**
 * `x-panel-keys` 나눠 실행한 결과 합치기: 작업대 블록(목록·격자·글…)은 전체 실행(`stage`)에서,
 * 패널 블록(`types`)·`optionHints`는 패널 실행(`panel`, 그 실행의 다른 블록도 그대로)에서.
 * 패널 실행이 아직 없으면 전체 실행 그대로, 전체 실행이 없으면 없음
 */
export function mergePanelPart(
  stage: Block[] | undefined,
  panel: Block[] | undefined,
  types: Block["type"][],
): Block[] | undefined {
  if (!stage || !panel) return stage
  return [
    ...stage.filter((b) => !types.includes(b.type) && b.type !== "optionHints"),
    ...panel,
  ]
}
