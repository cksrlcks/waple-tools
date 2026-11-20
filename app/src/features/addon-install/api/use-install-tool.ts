import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { interactiveKeys, toolKeys } from "@/entities/tool"
import { toolResourceKeys } from "@/entities/tool-resource"
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
    onSuccess: ({ failedResources }) => {
      if (failedResources.length)
        toast.warning("일부 항목을 받지 못했습니다. 목록에서 다시 받아 주세요.")
      else toast.success("설치했습니다.")
      qc.invalidateQueries({ queryKey: toolResourceKeys.list(toolId) })
      qc.invalidateQueries({ queryKey: interactiveKeys.all })
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
