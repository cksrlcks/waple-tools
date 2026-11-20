import type { Block } from "@/entities/tool"

/**
 * 결과 블록을 놓을 곳으로 나눈다: `list-detail`이면 리소스 목록은 왼쪽 열,
 * `panelTypes` 종류는 옵션 패널 아래, 나머지는 작업대.
 */
export function splitStageBlocks(
  blocks: Block[],
  mode: "list-detail" | null,
  panelTypes: Block["type"][],
): { list: Block[]; stage: Block[]; panel: Block[] } {
  const out = {
    list: [] as Block[],
    stage: [] as Block[],
    panel: [] as Block[],
  }
  for (const b of blocks) {
    if (mode === "list-detail" && b.type === "resourceList") out.list.push(b)
    else if (panelTypes.includes(b.type)) out.panel.push(b)
    else out.stage.push(b)
  }
  return out
}
