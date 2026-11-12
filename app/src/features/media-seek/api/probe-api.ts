import { invoke } from "@tauri-apps/api/core"

import type { Block } from "@/entities/tool"

/** 도구를 `{"x-probe": true}`로 실행해 길이·fps 블록을 받는다 (파일을 만들지 않는다) */
export function probeMedia(toolId: string, input: string): Promise<Block[]> {
  return invoke("run_interactive", {
    toolId,
    input,
    opts: { "x-probe": true },
  })
}
