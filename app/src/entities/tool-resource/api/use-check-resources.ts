import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { toUserMessage } from "@/shared/lib/to-user-message"

import { checkToolResources } from "./tool-resource-api"
import { toolResourceKeys } from "./tool-resource-keys"

/** [업데이트 확인]: 눌렀을 때만 네트워크를 쓴다 */
export function useCheckResources(toolId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => checkToolResources(toolId),
    onSuccess: (list) => {
      qc.setQueryData(toolResourceKeys.list(toolId), list)
      if (!list.some((r) => r.latest)) toast.success("모두 최신입니다.")
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
