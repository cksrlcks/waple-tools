import { useMemo } from "react"
import { useMutation } from "@tanstack/react-query"
import { toast } from "sonner"

import type { Settings } from "@/entities/settings"
import { toolIcon, type ToolEntry } from "@/entities/tool"
import { BlockList, runInteractive } from "@/features/interactive-result"
import { runButtonsField, schemaToFields } from "@/features/tool-options"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"

import { runButtonOpts } from "../lib/run-button-opts"
import { RunButtonCard } from "./run-button-card"

type Props = {
  tool: ToolEntry
  settings: Settings
  className?: string
}

/**
 * `x-run-buttons` 도구: 값마다 버튼 하나, 누를 때만 실행한다 (열 때·옵션 바꿀 때 자동 실행 없음).
 * 같은 버튼을 다시 눌러도 실행되도록 쿼리 캐시가 아니라 mutation.
 */
// ponytail: x-run-buttons 필드 말고 다른 옵션은 그리지 않는다. 그런 도구가 생기면 옵션 패널을 붙인다
export function RunButtonsWorkspace({ tool, className }: Props) {
  const fields = useMemo(
    () => schemaToFields(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const field = useMemo(
    () => runButtonsField(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const run = useMutation({
    mutationFn: (value: string) =>
      runInteractive(
        tool.id,
        null,
        runButtonOpts(fields, field?.key ?? "", value),
      ),
    onError: (err) => toast.error(toUserMessage(err)),
  })
  if (!field) return null
  const Icon = toolIcon(tool.id)

  return (
    <div className={cn("flex min-w-0 flex-1 flex-col", className)}>
      <section
        aria-label="작업대"
        className="bg-stage text-stage-foreground stage-plain m-2 flex min-w-0 flex-1 flex-col overflow-y-auto rounded-xl"
      >
        <div className="@container m-auto flex w-full max-w-xl flex-col gap-5 p-6">
          <header className="flex items-start gap-3">
            <span className="bg-primary/10 text-primary flex size-10 shrink-0 items-center justify-center rounded-xl">
              <Icon className="size-5" stroke={1.75} />
            </span>
            <div className="flex min-w-0 flex-col gap-1">
              <h2 className="text-base font-semibold">{tool.name}</h2>
              <p className="text-muted-foreground text-xs">
                {tool.description}
              </p>
            </div>
          </header>
          <div className="flex flex-col gap-2">
            <h3 className="text-muted-foreground text-xs font-semibold">
              {field.label}
            </h3>
            <div className="grid auto-rows-fr grid-cols-1 gap-2 @md:grid-cols-2">
              {field.options.map((o) => (
                <RunButtonCard
                  key={o.value}
                  label={o.label}
                  note={o.note}
                  active={run.isSuccess && run.variables === o.value}
                  pending={run.isPending && run.variables === o.value}
                  onClick={() => run.mutate(o.value)}
                />
              ))}
            </div>
          </div>
          {run.data && <BlockList blocks={run.data} icon={false} />}
        </div>
      </section>
    </div>
  )
}
