import { useEffect, useMemo, useRef, useState } from "react"

import { useSaveSettings, type Settings } from "@/entities/settings"
import type { FileResult, PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, InputList, statPaths } from "@/features/file-input"
import {
  canRun,
  PolicyPicker,
  PreviewPanel,
  ResultReport,
  RunBar,
  useJob,
} from "@/features/job-run"
import {
  mergeOptions,
  OptionsForm,
  requireAnyMissing,
  schemaToFields,
} from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { initialOptions } from "../lib/initial-options"
import { initialPolicy, suffixOf } from "../lib/initial-policy"

type Props = {
  tool: ToolEntry
  settings: Settings
  /** 실행·되돌리기 중인지 알린다 (그동안 사이드바 이동을 막는다) */
  onBusyChange?: (busy: boolean) => void
  className?: string
}

/** 파일을 처리하는 도구 화면: 입력 → 옵션·저장 방식 → 미리보기·실행 → 결과 */
export function FileToolWorkspace({
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
    initialOptions(
      fields,
      settings.lastOptions[tool.id],
      settings.keepMetadata,
    ),
  )
  const [policy, setPolicy] = useState(() =>
    initialPolicy(settings.defaultPolicy, tool.id),
  )
  const [recursive, setRecursive] = useState(false)
  const [items, setItems] = useState<PathStat[]>([])
  const [inputError, setInputError] = useState<string | null>(null)
  const job = useJob(tool.id)
  const save = useSaveSettings()

  const busy = job.busy
  useEffect(() => {
    onBusyChange?.(busy)
  }, [busy, onBusyChange])
  useEffect(() => () => onBusyChange?.(false), [onBusyChange])

  const inputs = items.map((i) => i.path)
  const check = canRun(inputs, policy, busy)
  // 도구가 "이 중 하나는 켜야 함"을 선언했으면 그것부터 확인 (예: 이미지 최적화의 단계 스위치)
  const optionsProblem = requireAnyMissing(tool.optionsSchema, options)
  const disabledReason = optionsProblem ?? (check.ok ? null : check.reason)
  const spec = {
    inputs,
    recursive,
    policy,
    keepIfLarger: settings.keepIfLarger,
    // 동시 작업 수는 항상 자동(CPU 코어 수 - 1). 설정에서 숨겼다
    threads: 0,
    // 폼 입력(빈 칸, 범위 밖, 소수)을 도구가 받을 수 있는 값으로 정리
    opts: mergeOptions(fields, options),
  }

  const add = (paths: string[]) => {
    const fresh = paths.filter((p) => !inputs.includes(p))
    if (fresh.length === 0) return
    setInputError(null)
    statPaths(fresh)
      .then((stats) =>
        setItems((cur) => [
          ...cur,
          ...stats.filter((s) => !cur.some((c) => c.path === s.path)),
        ]),
      )
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  // 실행·되돌리기 뒤에는 파일이 옮겨지므로 크기·존재 여부를 다시 읽는다
  const itemsRef = useRef(items)
  itemsRef.current = items
  const reportId = job.report?.jobId
  const undoItems = job.undoItems
  useEffect(() => {
    if (!reportId) return
    statPaths(itemsRef.current.map((i) => i.path))
      .then(setItems)
      .catch(() => {})
  }, [reportId, undoItems])

  const results = useMemo(
    () =>
      new Map<string, FileResult>(
        job.report?.files.map((f) => [f.input, f]) ?? [],
      ),
    [job.report],
  )

  const run = () => {
    save.mutate({
      ...settings,
      lastOptions: { ...settings.lastOptions, [tool.id]: spec.opts },
    })
    job.run(spec)
  }

  const error = inputError ?? job.error

  return (
    <section className={cn("flex flex-col gap-4", className)}>
      <header>
        <h1 className="text-base font-semibold">{tool.name}</h1>
        <p className="text-muted-foreground">{tool.description}</p>
      </header>

      <DropZone onAdd={add} onError={setInputError} disabled={busy} />
      <InputList
        items={items}
        results={results}
        onRemove={(p) => setItems((cur) => cur.filter((c) => c.path !== p))}
        onClear={() => setItems([])}
        disabled={busy}
      />

      <div className="grid grid-cols-2 gap-6">
        <section className="flex flex-col gap-2">
          <h2 className="text-2xs text-muted-foreground font-medium">옵션</h2>
          <OptionsForm
            fields={fields}
            value={options}
            onChange={setOptions}
            disabled={busy}
          />
        </section>
        <section className="flex flex-col gap-2">
          <h2 className="text-2xs text-muted-foreground font-medium">
            저장 방식
          </h2>
          <PolicyPicker
            value={policy}
            onChange={setPolicy}
            defaultSuffix={suffixOf(tool.id)}
            recursive={recursive}
            onRecursiveChange={setRecursive}
            disabled={busy}
          />
        </section>
      </div>

      <RunBar
        onPreview={() => job.preview(spec)}
        onRun={run}
        onCancel={job.cancel}
        running={job.running}
        previewing={job.previewing}
        progress={job.progress}
        disabledReason={disabledReason}
      />
      {error && <ErrorBanner message={error} className="rounded-md border" />}
      {job.plan && <PreviewPanel plan={job.plan} />}
      {job.report && (
        <ResultReport
          report={job.report}
          undoItems={job.undoItems}
          onUndo={job.undo}
          undoing={job.undoing}
        />
      )}
    </section>
  )
}
