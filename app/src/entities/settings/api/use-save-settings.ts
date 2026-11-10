import { useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@tauri-apps/api/core"

import type { Settings } from "../model/types"
import { settingsKeys } from "./settings-keys"

/** 설정 저장. 화면에는 바로 반영하고(낙관적), 실패하면 되돌린다. */
export function useSaveSettings() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (settings: Settings) =>
      invoke<void>("set_settings", { settings }),
    onMutate: async (next) => {
      await qc.cancelQueries({ queryKey: settingsKeys.all })
      const prev = qc.getQueryData<Settings>(settingsKeys.all)
      qc.setQueryData(settingsKeys.all, next)
      return { prev }
    },
    onError: (_err, _next, ctx) => {
      if (ctx?.prev) qc.setQueryData(settingsKeys.all, ctx.prev)
    },
    onSettled: () => qc.invalidateQueries({ queryKey: settingsKeys.all }),
  })
}
