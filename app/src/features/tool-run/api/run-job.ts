import { Channel, invoke } from "@tauri-apps/api/core"

import type { Progress, RunResult } from "@/entities/tool"

export function runJob(
  toolId: string,
  inputs: string[],
  onProgress: (p: Progress) => void,
): Promise<RunResult> {
  const channel = new Channel<Progress>()
  channel.onmessage = onProgress
  return invoke<RunResult>("run_job", {
    toolId,
    inputs,
    opts: {},
    onProgress: channel,
  })
}

export function cancelJob(): Promise<void> {
  return invoke("cancel_job")
}
