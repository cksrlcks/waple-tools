import { useState } from "react"
import { useMutation } from "@tanstack/react-query"

import type { JobSpec, Progress } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"

import {
  cancelJob,
  newJobId,
  previewJob,
  runJob,
  undoJob,
} from "../api/job-api"

/** 미리보기 → 실행(진행률·취소) → 되돌리기 흐름 */
export function useJob(toolId: string) {
  const [jobId, setJobId] = useState<string | null>(null)
  const [progress, setProgress] = useState<Progress | null>(null)
  const [cancelError, setCancelError] = useState<string | null>(null)

  const preview = useMutation({
    mutationFn: (spec: JobSpec) => previewJob(toolId, spec),
  })
  const run = useMutation({
    mutationFn: async (spec: JobSpec) => {
      const id = await newJobId()
      setJobId(id)
      return runJob(id, toolId, spec, setProgress)
    },
  })
  const undo = useMutation({ mutationFn: undoJob })

  const error =
    cancelError ??
    (preview.isError ? toUserMessage(preview.error) : null) ??
    (run.isError ? toUserMessage(run.error) : null) ??
    (undo.isError ? toUserMessage(undo.error) : null)

  return {
    plan: preview.data,
    previewing: preview.isPending,
    preview: (spec: JobSpec) => {
      run.reset()
      undo.reset()
      preview.mutate(spec)
    },
    report: run.data,
    running: run.isPending,
    progress,
    run: (spec: JobSpec) => {
      preview.reset()
      undo.reset()
      setCancelError(null)
      setProgress(null)
      run.mutate(spec)
    },
    cancel: () => {
      if (jobId)
        cancelJob(jobId).catch(() =>
          setCancelError("작업을 취소할 수 없습니다."),
        )
    },
    undoItems: undo.data,
    undoing: undo.isPending,
    undo: () => {
      if (run.data) undo.mutate(run.data.jobId)
    },
    error,
  }
}
