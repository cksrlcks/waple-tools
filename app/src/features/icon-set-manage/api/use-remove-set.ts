import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { iconSetKeys, removeIconSet } from "@/entities/icon-set"
import { toUserMessage } from "@/shared/lib/to-user-message"

export function useRemoveSet() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (prefix: string) => removeIconSet(prefix),
    onSuccess: (_, prefix) => {
      toast.success("세트를 삭제했습니다.")
      return Promise.all([
        qc.invalidateQueries({ queryKey: iconSetKeys.installed() }),
        qc.invalidateQueries({ queryKey: iconSetKeys.set(prefix) }),
      ])
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
