import { useMemo } from "react"
import { useMutation } from "@tanstack/react-query"
import { toast } from "sonner"

import type { Settings } from "@/entities/settings"
import type { ToolEntry } from "@/entities/tool"
import { BlockList, runInteractive } from "@/features/interactive-result"
import { runButtonsField, schemaToFields } from "@/features/tool-options"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { runButtonOpts } from "../lib/run-button-opts"

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

  return (
    <div className={cn("flex min-w-0 flex-1 flex-col", className)}>
      <section
        aria-label="작업대"
        className="bg-stage text-stage-foreground stage-plain m-2 flex min-w-0 flex-1 flex-col overflow-y-auto rounded-xl"
      >
        <div className="m-auto flex w-full max-w-md flex-col gap-3 p-6">
          <h2 className="text-sm font-medium">{field.label}</h2>
          <ul className="flex flex-col gap-2">
            {field.options.map((o) => (
              <li key={o.value} className="flex items-center gap-3">
                <Button
                  className="w-24 shrink-0"
                  disabled={run.isPending && run.variables === o.value}
                  onClick={() => run.mutate(o.value)}
                >
                  {o.label}
                </Button>
                {o.note && (
                  <span className="text-stage-muted text-sm">{o.note}</span>
                )}
              </li>
            ))}
          </ul>
          {run.data && <BlockList blocks={run.data} icon={false} />}
        </div>
      </section>
    </div>
  )
}
