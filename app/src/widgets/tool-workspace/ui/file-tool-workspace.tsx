import { useEffect, useMemo, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { useSaveSettings, type Settings } from "@/entities/settings"
import type { FileResult, PathStat, ToolEntry } from "@/entities/tool"
import {
  DropZone,
  FileList,
  InputList,
  moveItem,
  OrderGrid,
  statPaths,
} from "@/features/file-input"
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
import { NO_PAGES, PagePicker } from "@/features/page-picker"
import {
  allowsLarger,
  CompactSelect,
  isEnabled,
  isSingleInput,
  mergeOptions,
  OptionsForm,
  pagePickerKey,
  perFileFields,
  regionKeys,
  reportsSize,
  requireAnyMissing,
  schemaToFields,
} from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { buildRegion } from "../lib/build-region"
import { filterSupported } from "../lib/filter-supported"
import { initialOptions } from "../lib/initial-options"
import { initialPolicy, suffixOf } from "../lib/initial-policy"
import { inputExts } from "../lib/input-exts"

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
  // 작업대에서 페이지를 고르는 도구(x-page-picker): 고른 쪽이 옵션 값
  const pickerKey = pagePickerKey(tool.optionsSchema)
  // 그 옵션이 지금 비활성이면(예: 분할의 "페이지마다") 썸네일은 보기만 한다
  const pickerField = fields.find((f) => f.key === pickerKey)
  const pickable =
    pickerKey !== null && (!pickerField || isEnabled(pickerField, options))
  const optionsProblem =
    requireAnyMissing(tool.optionsSchema, options) ??
    (pickable && pickerKey && options[pickerKey] === NO_PAGES
      ? "변환할 페이지를 고르세요."
      : null)
  // 실행할 수 없는 이유는 누를 때 토스트로만 알린다
  const blockedReason = optionsProblem ?? (check.ok ? null : check.reason)
  // 파일마다 고르는 옵션 (x-per-file): 파일 목록 줄에서 고르고, 실행할 때 파일별 값으로 보낸다
  const perFile = useMemo(
    () => perFileFields(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const perFileKeys = Object.keys(perFile)
  const [fileOpts, setFileOpts] = useState<
    Record<string, Record<string, string>>
  >({})
  /** 고른 값 → 확장자별 처음 값 → 옵션 기본값 */
  const fileValue = (item: PathStat, key: string): string => {
    const ext = /\.([^.\\/]+)$/.exec(item.path)?.[1]?.toLowerCase() ?? ""
    return (
      fileOpts[item.path]?.[key] ??
      perFile[key]?.[ext] ??
      String(options[key] ?? "")
    )
  }
  // 폼 입력(빈 칸, 범위 밖, 소수)을 도구가 받을 수 있는 값으로 정리
  const baseOpts = mergeOptions(fields, options)
  const spec = {
    inputs,
    recursive,
    policy,
    // 커지는 것이 정상인 도구(x-allow-larger)는 "커지면 원본 유지"를 쓰지 않는다
    keepIfLarger:
      settings.keepIfLarger && !allowsLarger(tool.optionsSchema, baseOpts),
    // 동시 작업 수는 항상 자동(CPU 코어 수 - 1). 설정에서 숨겼다
    threads: 0,
    opts:
      perFileKeys.length === 0
        ? baseOpts
        : {
            ...baseOpts,
            "x-per-file": Object.fromEntries(
              items
                .filter((i) => !i.isDir)
                .map((i) => [
                  i.path,
                  Object.fromEntries(
                    perFileKeys.map((k) => [k, fileValue(i, k)]),
                  ),
                ]),
            ),
          },
  }

  // 한 파일만 받는 도구(x-single-input): 넣으면 바꿔치기, 폴더는 받지 않는다
  const single = isSingleInput(tool.optionsSchema)
  // 받지 않는 형식은 넣지 않고 알린다 (실행 단계와 같은 규칙)
  const exts = tool.extensions
  const warnRejected = (n: number) => {
    if (n > 0) toast.error(`지원하지 않는 형식이라 ${n}개를 넣지 않았습니다.`)
  }

  const addSingle = (paths: string[]) => {
    if (paths.length === 0) return
    setInputError(null)
    statPaths(paths)
      .then((stats) => {
        const files = stats.filter((s) => !s.isDir)
        const { ok } = filterSupported(files, exts)
        const first = ok[0]
        if (!first) {
          toast.error(
            files.length === 0
              ? "이 도구는 폴더가 아닌 파일 하나를 받습니다."
              : "지원하지 않는 형식입니다.",
          )
          return
        }
        if (paths.length > 1) toast.info("한 번에 한 파일만 처리합니다.")
        setItems([first])
      })
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  const add = (paths: string[]) => {
    if (single) return addSingle(paths)
    const fresh = paths.filter((p) => !inputs.includes(p))
    if (fresh.length === 0) return
    setInputError(null)
    statPaths(fresh)
      .then((stats) => {
        const { ok, rejected } = filterSupported(stats, exts)
        warnRejected(rejected)
        setItems((cur) => [
          ...cur,
          ...ok.filter((s) => !cur.some((c) => c.path === s.path)),
        ])
      })
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  const removeItem = (p: string) =>
    setItems((cur) => cur.filter((c) => c.path !== p))

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

  // 다른 파일로 바꾸면 고른 쪽을 지운다 (쪽 번호는 파일마다 다르다)
  useEffect(() => {
    if (pickerKey) setOptions((o) => ({ ...o, [pickerKey]: "" }))
  }, [pickerKey, previewPath])

  // 미리보기 위에서 끌어 정하는 영역 (x-region)
  const keys = useMemo(
    () => regionKeys(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const region = buildRegion(keys, options, (patch) =>
    setOptions((o) => ({ ...o, ...patch })),
  )

  // 여러 파일을 하나로 합치는 도구: 결과 하나를 넣은 파일 모두에 보여 준다
  const combine = tool.kind === "combine"
  const results = useMemo(() => {
    const files = job.report?.files ?? []
    const all = combine && files[0]
    return new Map<string, FileResult>(
      all
        ? itemsRef.current.map((i) => [i.path, all])
        : files.map((f) => [f.input, f]),
    )
  }, [job.report, combine])
  const mode = previewMode(tool.optionsSchema, options)
  // 미리보기도 격자·썸네일도 없는 도구: 작업대에 파일 목록만 (예: 폰트 변환, PDF 압축)
  const listOnly = mode === "none" && !combine && !pickerKey

  const run = () => {
    save.mutate({
      ...settings,
      // 파일별 값(x-per-file)은 파일마다 달라서 기억하지 않는다
      lastOptions: { ...settings.lastOptions, [tool.id]: baseOpts },
    })
    job.run(spec)
  }

  // 옵션 패널에서 빼는 옵션
  const hiddenKeys = [
    // 영역은 미리보기에서 끌어 정한다
    ...(region && keys ? [keys.x, keys.y, keys.w, keys.h] : []),
    // 고른 쪽은 작업대의 썸네일로 정한다
    ...(pickerKey ? [pickerKey] : []),
    // 파일마다 고르는 옵션은 파일 목록에 있다
    ...perFileKeys,
  ]

  const error = inputError ?? job.error

  return (
    <div className={cn("flex min-w-0 flex-1 flex-col", className)}>
      <div className="flex min-h-0 flex-1">
        {/* 작업대: 미리보기가 주인공, 넣은 파일은 아래 필름 띠 */}
        <section
          aria-label="작업대"
          className="bg-stage text-stage-foreground my-2 ml-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl"
        >
          {items.length === 0 ? (
            <DropZone
              onAdd={add}
              onError={setInputError}
              single={single}
              extensions={exts}
              disabled={busy}
            />
          ) : (
            <>
              {combine ? (
                // 합치는 도구: 작업대 격자에서 순서를 정한다 (아래 필름 띠는 없음)
                <OrderGrid
                  className="min-h-0 flex-1"
                  items={items}
                  results={results}
                  onReorder={(from, to) =>
                    setItems((cur) => moveItem(cur, from, to))
                  }
                  onRemove={removeItem}
                  onClear={() => setItems([])}
                  renderThumb={(item) => <Thumb item={item} />}
                  trailing={
                    <DropZone
                      variant="tile"
                      className="h-40 w-full"
                      single={single}
                      extensions={exts}
                      onAdd={add}
                      onError={setInputError}
                      disabled={busy}
                    />
                  }
                  disabled={busy}
                />
              ) : pickerKey ? (
                <PagePicker
                  className="min-h-0 flex-1"
                  toolId={tool.id}
                  path={previewPath}
                  value={String(options[pickerKey] ?? "")}
                  selectable={pickable}
                  onChange={(v) =>
                    setOptions((o) => ({ ...o, [pickerKey]: v }))
                  }
                  disabled={busy}
                />
              ) : listOnly ? (
                <FileList
                  className="min-h-0 flex-1"
                  items={items}
                  results={results}
                  onRemove={removeItem}
                  onClear={() => setItems([])}
                  renderThumb={(item) => <Thumb item={item} />}
                  renderExtra={(item) =>
                    item.isDir
                      ? null
                      : perFileKeys.map((k) => {
                          const field = fields.find((f) => f.key === k)
                          if (field?.kind !== "select") return null
                          return (
                            <CompactSelect
                              key={k}
                              className="w-40 shrink-0"
                              field={field}
                              value={fileValue(item, k)}
                              onChange={(v) =>
                                setFileOpts((cur) => ({
                                  ...cur,
                                  [item.path]: { ...cur[item.path], [k]: v },
                                }))
                              }
                              disabled={busy}
                            />
                          )
                        })
                  }
                  trailing={
                    <DropZone
                      variant="tile"
                      className="h-10 w-full"
                      single={single}
                      extensions={exts}
                      onAdd={add}
                      onError={setInputError}
                      disabled={busy}
                    />
                  }
                  disabled={busy}
                />
              ) : (
                <PreviewPane
                  className="min-h-0 flex-1"
                  toolId={tool.id}
                  path={previewPath}
                  opts={spec.opts}
                  blockedReason={optionsProblem}
                  mode={mode}
                  region={busy ? null : region}
                />
              )}
              {!combine && !listOnly && (
                <InputList
                  items={items}
                  results={results}
                  onRemove={removeItem}
                  onClear={() => setItems([])}
                  renderThumb={(item) => <Thumb item={item} />}
                  selected={previewPath}
                  onSelect={setSelected}
                  trailing={
                    <DropZone
                      variant="tile"
                      single={single}
                      extensions={exts}
                      onAdd={add}
                      onError={setInputError}
                      disabled={busy}
                    />
                  }
                  disabled={busy}
                />
              )}
            </>
          )}
        </section>

        {/* 옵션 패널: 스크롤 */}
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
              {/* 패널에 보일 옵션이 없으면(모두 작업대·파일 목록에서 정함) 빈 칸을 그리지 않는다 */}
              {fields.some((f) => !hiddenKeys.includes(f.key)) && (
                <OptionsForm
                  className="py-4"
                  fields={fields}
                  value={options}
                  onChange={setOptions}
                  disabled={busy}
                  hidden={hiddenKeys}
                  // 넣은 파일 형식에 따라 보이는 옵션이 있다 (예: PNG 색상 수)
                  context={{ $inputExt: inputExts(items, tool.extensions) }}
                />
              )}
              <section className="flex flex-col gap-3 py-4">
                <h3 className="text-xs font-semibold">저장 위치</h3>
                <PolicyPicker
                  value={policy}
                  onChange={setPolicy}
                  defaultSuffix={suffixOf(tool.id)}
                  recursive={recursive}
                  onRecursiveChange={setRecursive}
                  showRecursive={items.some((i) => i.isDir)}
                  // 합치는 도구도 원본은 늘 그대로라 "원본 옆"(접미사)이 없다
                  generator={tool.kind === "generator" || combine}
                  disabled={busy}
                />
              </section>
            </div>

            {error && (
              <ErrorBanner message={error} className="rounded-lg border" />
            )}
          </div>
        </aside>
      </div>

      {/* 하단 전체 폭 막대: 왼쪽 결과·대상 요약, 오른쪽 실행 */}
      <RunBar
        className="border-t py-3 pr-6 pl-4"
        onPreview={() => job.preview(spec)}
        onRun={run}
        onCancel={job.cancel}
        running={job.running}
        previewing={job.previewing}
        progress={job.progress}
        cancelling={job.cancelling}
        disabled={items.length === 0}
        blockedReason={blockedReason}
      >
        {job.report ? (
          <ResultReport
            report={job.report}
            undoItems={job.undoItems}
            onUndo={job.undo}
            undoing={job.undoing}
            generator={tool.kind === "generator"}
            showSize={
              tool.kind !== "generator" || reportsSize(tool.optionsSchema)
            }
          />
        ) : (
          job.plan && <PreviewPanel plan={job.plan} />
        )}
      </RunBar>
    </div>
  )
}
