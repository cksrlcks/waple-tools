import { useState } from "react"
import { useMutation } from "@tanstack/react-query"
import { open } from "@tauri-apps/plugin-dialog"

import type { Progress as JobProgress, ToolInfo } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { Progress } from "@/shared/ui/progress"

import { cancelJob, runJob } from "../api/run-job"
import { FileResultRow } from "./file-result-row"

type Props = {
  tool: ToolInfo
  className?: string
}

export function ToolRunner({ tool, className }: Props) {
  const [inputs, setInputs] = useState<string[]>([])
  const [progress, setProgress] = useState<JobProgress | null>(null)
  const run = useMutation({
    mutationFn: () => runJob(tool.meta.id, inputs, setProgress),
  })

  const pickFiles = async () => {
    const picked = await open({ multiple: true }).catch(() => null)
    if (picked) setInputs(picked)
  }

  const percent = progress
    ? ((progress.fileIndex + progress.ratio) / progress.total) * 100
    : 0

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
        <Button
          onClick={() => {
            setProgress(null)
            run.mutate()
          }}
          disabled={inputs.length === 0 || run.isPending}
        >
          실행
        </Button>
        {run.isPending && (
          <Button variant="outline" onClick={() => cancelJob()}>
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

      {run.isError && <ErrorBanner message={toUserMessage(run.error)} />}

      {run.data && (
        <ul className="flex flex-col gap-1 text-xs">
          {run.data.files.map((f) => (
            <FileResultRow key={f.input} result={f} />
          ))}
        </ul>
      )}
    </section>
  )
}
