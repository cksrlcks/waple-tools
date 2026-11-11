import { invoke } from "@tauri-apps/api/core"

import type { Block } from "@/entities/tool"

export function runInteractive(
  toolId: string,
  input: string,
  opts: Record<string, unknown>,
): Promise<Block[]> {
  return invoke("run_interactive", { toolId, input, opts })
}
