import { invoke } from "@tauri-apps/api/core"

import type { PathStat } from "@/entities/tool"

export function statPaths(paths: string[]): Promise<PathStat[]> {
  return invoke("stat_paths", { paths })
}
