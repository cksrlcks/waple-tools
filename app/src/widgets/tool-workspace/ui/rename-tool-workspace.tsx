import { useEffect, useMemo, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"

import { useSaveSettings, type Settings } from "@/entities/settings"
import type { JobSpec, PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, statPaths } from "@/features/file-input"
import {
  renameKeys,
  renameSummary,
  RenameTable,
  useRenamePlan,
} from "@/features/file-rename"
import { ResultReport, RunBar, useJob } from "@/features/job-run"
import {
  mergeOptions,
  OptionsForm,
  schemaToFields,
} from "@/features/tool-options"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Checkbox } from "@/shared/ui/checkbox"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { initialOptions } from "../lib/initial-options"
import { remapRenamed } from "../lib/remap-renamed"

type Props = {
  tool: ToolEntry
  settings: Settings
  onBusyChange?: (busy: boolean) => void
  className?: string
}

/** 파일명 일괄 변경 화면: 입력 → 규칙 → 변경 전/후 표(충돌 시 실행 불가) → 이름 바꾸기 → 되돌리기 */
export function RenameToolWorkspace({
  tool,
  settings,
  onBusyChange,
  className,
}: Props) {
  const fields = useMemo(
    () => schemaToFields(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const [options, setOptions] = useState(() =>
    initialOptions(fields, settings.lastOptions[tool.id], false),
  )
  const [recursive, setRecursive] = useState(false)
  const [items, setItems] = useState<PathStat[]>([])
  const [inputError, setInputError] = useState<string | null>(null)
  const job = useJob(tool.id)
  const save = useSaveSettings()
  const qc = useQueryClient()

  const busy = job.busy
  useEffect(() => {
    onBusyChange?.(busy)
  }, [busy, onBusyChange])
  useEffect(() => () => onBusyChange?.(false), [onBusyChange])

  const inputs = items.map((i) => i.path)
  const opts = mergeOptions(fields, options)
  // 원본 이름을 제자리에서 바꾼다: 저장 위치·백업·용량 비교는 쓰지 않는다
  const spec: JobSpec = {
    inputs,
    recursive,
    policy: { type: "InPlaceWithBackup" },
    keepIfLarger: false,
    threads: 0,
    opts,
  }
  const plan = useRenamePlan(tool.id, spec)
  const s = renameSummary(plan.data, plan.current)

  // 바꾼 뒤엔 넣은 파일을 새 이름으로, 되돌린 뒤엔 원래 이름으로 따라간다 (표를 다시 만든다)
  const { report, undoItems } = job
  // 작업마다 "바꿈"·"되돌림"을 한 번씩만 반영한다 (새 실행 직전 되돌리기 결과가 지워질 때 다시 바꾸지 않게)
  const applied = useRef(new Set<string>())
  useEffect(() => {
    if (!report) return
    const step = `${report.jobId}:${undoItems ? "undo" : "run"}`
    if (applied.current.has(step)) return
    applied.current.add(step)
    setItems((cur) => {
      const next = remapRenamed(
        cur.map((i) => i.path),
        report,
        undoItems,
      )
      return cur.map((i, n) => ({ ...i, path: next[n] ?? i.path }))
    })
    void qc.invalidateQueries({ queryKey: renameKeys.all })
  }, [report, undoItems, qc])

  const add = (paths: string[]) => {
    const fresh = paths.filter((p) => !inputs.includes(p))
    if (fresh.length === 0) return
    setInputError(null)
    statPaths(fresh)
      .then((stats) =>
        setItems((cur) => [
          ...cur,
          ...stats.filter((st) => !cur.some((c) => c.path === st.path)),
        ]),
      )
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  const run = () => {
    save.mutate({
      ...settings,
      lastOptions: { ...settings.lastOptions, [tool.id]: opts },
    })
    job.run(spec)
  }

  const error =
    inputError ?? (plan.isError ? toUserMessage(plan.error) : null) ?? job.error

  return (
    <div className={cn("flex min-w-0 flex-1 flex-col", className)}>
      <div className="flex min-h-0 flex-1">
        <section
          aria-label="작업대"
          className="bg-stage text-stage-foreground my-2 ml-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl"
        >
          {items.length === 0 ? (
            <DropZone onAdd={add} onError={setInputError} disabled={busy} />
          ) : (
            <RenameTable
              className="min-h-0 flex-1"
              rows={plan.data?.rows}
              inputs={inputs}
              onRemove={(p) =>
                setItems((cur) => cur.filter((c) => c.path !== p))
              }
              onClear={() => setItems([])}
              summary={
                plan.data && (
                  <span className="tabular-nums">
                    바뀜 {s.changed} · 그대로 {s.unchanged} ·{" "}
                    <span className={cn(s.conflict > 0 && "text-destructive")}>
                      충돌 {s.conflict}
                    </span>
                  </span>
                )
              }
              trailing={
                <DropZone
                  variant="tile"
                  className="h-10 w-full"
                  onAdd={add}
                  onError={setInputError}
                  disabled={busy}
                />
              }
              disabled={busy}
            />
          )}
        </section>

        <aside
          aria-label="옵션"
          className="flex w-80 shrink-0 flex-col xl:w-88"
        >
          <div className="flex min-h-0 flex-1 scrollbar-gutter-stable flex-col gap-5 overflow-y-auto px-6 pt-4 pb-5">
            <header className="min-w-0">
              <h1 className="text-base font-semibold">{tool.name}</h1>
              <p className="text-muted-foreground mt-1 text-xs">
                {tool.description}
              </p>
            </header>

            <div className="divide-border flex flex-col divide-y border-t">
              <OptionsForm
                className="py-4"
                fields={fields}
                value={options}
                onChange={setOptions}
                disabled={busy}
              />
              {items.some((i) => i.isDir) && (
                <label className="flex items-center gap-2 py-4 text-sm">
                  <Checkbox
                    checked={recursive}
                    onCheckedChange={(c) => setRecursive(c === true)}
                    disabled={busy}
                  />
                  하위 폴더 포함
                </label>
              )}
            </div>

            {error && (
              <ErrorBanner message={error} className="rounded-lg border" />
            )}
          </div>
        </aside>
      </div>

      <RunBar
        className="border-t py-3 pr-6 pl-4"
        onRun={run}
        runLabel="이름 바꾸기"
        onCancel={job.cancel}
        running={job.running}
        previewing={false}
        progress={job.progress}
        cancelling={job.cancelling}
        disabled={items.length === 0 || busy || s.blockedReason !== null}
        blockedReason={s.blockedReason}
      >
        {job.report ? (
          <ResultReport
            report={job.report}
            undoItems={job.undoItems}
            onUndo={job.undo}
            undoing={job.undoing}
            showSize={false}
          />
        ) : (
          items.length > 0 && (
            <p className="text-muted-foreground truncate text-sm">
              {s.blockedReason ?? `${s.changed}개 파일의 이름을 바꿉니다.`}
            </p>
          )
        )}
      </RunBar>
    </div>
  )
}
