import { invoke } from "@tauri-apps/api/core"

import type { ToolPreview } from "@/entities/tool"

export function thumbnail(path: string): Promise<string> {
  return invoke("thumbnail", { path })
}

export function previewTool(
  toolId: string,
  input: string,
  opts: Record<string, unknown>,
): Promise<ToolPreview> {
  return invoke("preview_tool", { toolId, input, opts })
}
