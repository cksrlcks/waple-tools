import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { iconSetKeys, installIconSet } from "@/entities/icon-set"
import { toUserMessage } from "@/shared/lib/to-user-message"

export function useInstallSet() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (prefix: string) => installIconSet(prefix),
    onSuccess: (set) => {
      toast.success(`${set.name} 세트를 받았습니다.`)
      return qc.invalidateQueries({ queryKey: iconSetKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
