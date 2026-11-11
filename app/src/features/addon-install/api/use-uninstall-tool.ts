import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { toolKeys } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"

import { uninstallTool } from "./addon-api"

export function useUninstallTool(toolId: string) {
  const qc = useQueryClient()
  const mutation = useMutation({
    mutationFn: () => uninstallTool(toolId),
    onSuccess: () => {
      toast.success("제거했습니다.")
      return qc.invalidateQueries({ queryKey: toolKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
  return {
    uninstall: () => mutation.mutate(),
    uninstalling: mutation.isPending,
  }
}
