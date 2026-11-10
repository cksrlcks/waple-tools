import { useEffect, useMemo, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"

import { useSaveSettings, type Settings } from "@/entities/settings"
import type { FileResult, PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, InputList, statPaths } from "@/features/file-input"
import {
  previewKeys,
  previewMode,
  PreviewPane,
  Thumb,
} from "@/features/image-preview"
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
  const qc = useQueryClient()
  useEffect(() => {
    if (!reportId) return
    statPaths(itemsRef.current.map((i) => i.path))
      .then(setItems)
      .catch(() => {})
    // 같은 경로에 결과가 생겼거나 원본이 돌아왔으므로 썸네일·미리보기도 다시 만든다
    void qc.invalidateQueries({ queryKey: previewKeys.thumbnails })
    void qc.invalidateQueries({ queryKey: previewKeys.tools })
  }, [reportId, undoItems, qc])

  // 미리보기 대상: 고른 파일, 없거나 목록에서 빠지면 첫 번째 파일 (폴더 제외)
  const [selected, setSelected] = useState<string | null>(null)
  const files = items.filter((i) => !i.isDir && i.exists)
  const previewPath =
    files.find((i) => i.path === selected)?.path ?? files[0]?.path ?? null

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
    <div className={cn("flex min-w-0 flex-1", className)}>
      {/* 작업대: 미리보기가 주인공, 넣은 파일은 아래 필름 띠 */}
      <section
        aria-label="작업대"
        className="bg-stage text-stage-foreground my-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl"
      >
        {items.length === 0 ? (
          <DropZone onAdd={add} onError={setInputError} disabled={busy} />
        ) : (
          <>
            <PreviewPane
              className="min-h-0 flex-1"
              toolId={tool.id}
              path={previewPath}
              opts={spec.opts}
              blockedReason={optionsProblem}
              mode={previewMode(tool.optionsSchema)}
            />
            <InputList
              items={items}
              results={results}
              onRemove={(p) =>
                setItems((cur) => cur.filter((c) => c.path !== p))
              }
              onClear={() => setItems([])}
              renderThumb={(item) => <Thumb item={item} />}
              selected={previewPath}
              onSelect={setSelected}
              trailing={
                <DropZone
                  variant="tile"
                  onAdd={add}
                  onError={setInputError}
                  disabled={busy}
                />
              }
              disabled={busy}
            />
          </>
        )}
      </section>

      {/* 옵션 패널: 위는 스크롤, 실행 막대는 아래 고정 */}
      <aside aria-label="옵션" className="flex w-80 shrink-0 flex-col xl:w-88">
        <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 pt-4 pb-5">
          <header className="min-w-0">
            <h1 className="text-base font-semibold">{tool.name}</h1>
            <p className="text-muted-foreground text-xs">{tool.description}</p>
          </header>

          <div className="divide-border flex flex-col divide-y border-t">
            <OptionsForm
              className="py-4"
              fields={fields}
              value={options}
              onChange={setOptions}
              disabled={busy}
            />
            <section className="flex flex-col gap-3 py-4">
              <h3 className="text-xs font-semibold">저장 위치</h3>
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

          {error && (
            <ErrorBanner message={error} className="rounded-lg border" />
          )}
          {job.plan && <PreviewPanel plan={job.plan} />}
          {job.report && (
            <ResultReport
              report={job.report}
              undoItems={job.undoItems}
              onUndo={job.undo}
              undoing={job.undoing}
            />
          )}
        </div>
        <RunBar
          className="border-t px-5 py-3.5"
          onPreview={() => job.preview(spec)}
          onRun={run}
          onCancel={job.cancel}
          running={job.running}
          previewing={job.previewing}
          progress={job.progress}
          disabledReason={disabledReason}
        />
      </aside>
    </div>
  )
}
