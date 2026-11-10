import { useState } from "react"
import { useMutation } from "@tanstack/react-query"
import { open } from "@tauri-apps/plugin-dialog"

import type { Progress as JobProgress, ToolInfo } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { Progress } from "@/shared/ui/progress"

import { cancelJob, newJobId, runJob, undoJob } from "../api/run-job"
import { defaultSpec } from "../lib/default-spec"
import { summarizeUndo } from "../lib/summarize-undo"
import { FileResultRow } from "./file-result-row"

type Props = {
  tool: ToolInfo
  className?: string
}

export function ToolRunner({ tool, className }: Props) {
  const [inputs, setInputs] = useState<string[]>([])
  const [jobId, setJobId] = useState<string | null>(null)
  const [progress, setProgress] = useState<JobProgress | null>(null)
  const [error, setError] = useState<string | null>(null)
  const run = useMutation({
    mutationFn: async () => {
      const id = await newJobId()
      setJobId(id)
      return runJob(id, tool.meta.id, defaultSpec(inputs), setProgress)
    },
  })
  const undo = useMutation({ mutationFn: undoJob })

  const pickFiles = async () => {
    setError(null)
    try {
      const picked = await open({ multiple: true })
      if (picked) setInputs(picked)
    } catch {
      setError("파일을 선택할 수 없습니다.")
    }
  }

  const start = () => {
    setError(null)
    setProgress(null)
    undo.reset()
    run.mutate()
  }

  const cancel = () => {
    if (jobId)
      cancelJob(jobId).catch(() => setError("작업을 취소할 수 없습니다."))
  }

  const percent = progress?.total ? (progress.done / progress.total) * 100 : 0
  const shownError =
    error ??
    (run.isError ? toUserMessage(run.error) : null) ??
    (undo.isError ? toUserMessage(undo.error) : null)

  return (
    <section className={cn("flex flex-col gap-3", className)}>
      <header>
        <h1 className="text-base font-semibold">{tool.meta.name}</h1>
        <p className="text-muted-foreground">{tool.meta.description}</p>
      </header>

      <div className="flex gap-2">
        <Button variant="outline" onClick={pickFiles} disabled={run.isPending}>
          파일 선택
        </Button>
        <Button onClick={start} disabled={inputs.length === 0 || run.isPending}>
          실행
        </Button>
        {run.isPending && (
          <Button variant="outline" onClick={cancel}>
            취소
          </Button>
        )}
      </div>

      {inputs.length > 0 && (
        <ul className="text-muted-foreground text-xs">
          {inputs.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      )}

      {(run.isPending || progress) && <Progress value={percent} />}

      {shownError && <ErrorBanner message={shownError} />}

      {run.data && (
        <>
          <ul className="flex flex-col gap-1 text-xs">
            {run.data.files.map((f) => (
              <FileResultRow key={f.input} result={f} />
            ))}
          </ul>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              onClick={() => undo.mutate(run.data.jobId)}
              disabled={undo.isPending || undo.isSuccess}
            >
              되돌리기
            </Button>
            {undo.data && (
              <span className="text-muted-foreground text-xs">
                {summarizeUndo(undo.data)}
              </span>
            )}
          </div>
        </>
      )}
    </section>
  )
}
