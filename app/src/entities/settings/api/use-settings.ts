import { useQuery } from "@tanstack/react-query"
import { invoke } from "@tauri-apps/api/core"

import type { Settings } from "../model/types"
import { settingsKeys } from "./settings-keys"

export function useSettings() {
  return useQuery({
    queryKey: settingsKeys.all,
    queryFn: () => invoke<Settings>("get_settings"),
    staleTime: Infinity,
  })
}
