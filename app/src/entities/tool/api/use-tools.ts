import { useQuery } from "@tanstack/react-query"
import { invoke } from "@tauri-apps/api/core"

import type { ToolInfo } from "../model/types"
import { toolKeys } from "./tool-keys"

export function useTools() {
  return useQuery({
    queryKey: toolKeys.list(),
    queryFn: () => invoke<ToolInfo[]>("list_tools"),
  })
}
