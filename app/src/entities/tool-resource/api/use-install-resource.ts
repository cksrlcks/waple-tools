import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { interactiveKeys } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"

import { installToolResource } from "./tool-resource-api"
import { toolResourceKeys } from "./tool-resource-keys"

/** 성공하면 목록과 대화형 결과를 다시 가져온다 */
export function useInstallResource(toolId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => installToolResource(toolId, id),
    onSuccess: () => {
      toast.success("받았습니다.")
      qc.invalidateQueries({ queryKey: toolResourceKeys.list(toolId) })
      qc.invalidateQueries({ queryKey: interactiveKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
