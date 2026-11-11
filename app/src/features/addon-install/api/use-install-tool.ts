import { useState } from "react"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { toolKeys, type DownloadProgress } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"

import { installTool } from "./addon-api"

export function useInstallTool(toolId: string) {
  const qc = useQueryClient()
  const [progress, setProgress] = useState<DownloadProgress | null>(null)
  const mutation = useMutation({
    mutationFn: () => {
      setProgress(null)
      return installTool(toolId, setProgress)
    },
    onSuccess: () => {
      toast.success("설치했습니다.")
      return qc.invalidateQueries({ queryKey: toolKeys.all })
    },
    onError: (err) => toast.error(toUserMessage(err)),
  })
  return {
    install: () => mutation.mutate(),
    installing: mutation.isPending,
    progress,
  }
}
