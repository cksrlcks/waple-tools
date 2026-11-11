import { invoke } from "@tauri-apps/api/core"

import type { Block } from "@/entities/tool"

export function runInteractive(
  toolId: string,
  /** `x-input: "none"` 도구는 null */
  input: string | null,
  opts: Record<string, unknown>,
): Promise<Block[]> {
  return invoke("run_interactive", { toolId, input, opts })
}

/** 결과 블록을 새 파일로 저장한다 (같은 이름이 있으면 `이름 (1).확장자`). 실제로 저장한 경로 */
export function saveBlock(
  path: string,
  content: string,
  base64: boolean,
): Promise<string> {
  return invoke("save_block", { path, content, base64 })
}
