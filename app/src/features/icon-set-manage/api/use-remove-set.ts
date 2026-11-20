import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { iconSetKeys, removeIconSet } from "@/entities/icon-set"
import { toUserMessage } from "@/shared/lib/to-user-message"

export function useRemoveSet() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (prefix: string) => removeIconSet(prefix),
    onSuccess: () => {
      toast.success("세트를 삭제했습니다.")
      return qc.invalidateQueries({ queryKey: iconSetKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
