import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { toolKeys } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"

import {
  isInstalling,
  setInstallState,
  useInstallState,
} from "../model/install-state"
import { installTool } from "./addon-api"

export function useInstallTool(toolId: string) {
  const qc = useQueryClient()
  const { installing, progress } = useInstallState(toolId)
  const mutation = useMutation({
    mutationFn: () => {
      setInstallState(toolId, null)
      return installTool(toolId, (p) => setInstallState(toolId, p)).finally(
        () => setInstallState(toolId, undefined),
      )
    },
    onSuccess: () => {
      toast.success("설치했습니다.")
      return qc.invalidateQueries({ queryKey: toolKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
  return {
    install: () => {
      if (!isInstalling(toolId)) mutation.mutate()
    },
    installing,
    progress,
  }
}
