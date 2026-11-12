import { invoke } from "@tauri-apps/api/core"

import type { JobSpec, RenamePlan } from "@/entities/tool"

/** 변경 전/후 미리보기 (파일은 그대로). 실행은 job-run의 `run_job` */
export function planRename(toolId: string, spec: JobSpec): Promise<RenamePlan> {
  return invoke("plan_rename", { toolId, spec })
}
