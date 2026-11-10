import { Channel, invoke } from "@tauri-apps/api/core"

import type {
  JobReport,
  JobSpec,
  Plan,
  Progress,
  UndoItem,
} from "@/entities/tool"

export function newJobId(): Promise<string> {
  return invoke("new_job_id")
}

export function previewJob(toolId: string, spec: JobSpec): Promise<Plan> {
  return invoke("preview_job", { toolId, spec })
}

export function runJob(
  jobId: string,
  toolId: string,
  spec: JobSpec,
  onProgress: (p: Progress) => void,
): Promise<JobReport> {
  const channel = new Channel<Progress>()
  channel.onmessage = onProgress
  return invoke("run_job", { jobId, toolId, spec, onProgress: channel })
}

export function cancelJob(jobId: string): Promise<void> {
  return invoke("cancel_job", { jobId })
}

export function undoJob(jobId: string): Promise<UndoItem[]> {
  return invoke("undo_job", { jobId })
}
