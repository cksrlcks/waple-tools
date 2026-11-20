import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { checkIconUpdates, iconSetKeys } from "@/entities/icon-set"
import { toUserMessage } from "@/shared/lib/to-user-message"

export function useCheckUpdates() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: checkIconUpdates,
    onSuccess: (sets) => {
      qc.setQueryData(iconSetKeys.installed(), sets)
      const n = sets.filter((s) => s.latest && s.latest !== s.version).length
      if (n === 0) toast.success("모두 최신입니다.")
      else toast.info(`새 버전이 있는 세트 ${n}개`)
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
}
