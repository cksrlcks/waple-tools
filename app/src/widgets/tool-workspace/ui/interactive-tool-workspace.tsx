import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type CSSProperties,
} from "react"
import { toast } from "sonner"

import type { Settings } from "@/entities/settings"
import type { PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, statPaths } from "@/features/file-input"
import { Thumb } from "@/features/image-preview"
import {
  applyOption,
  BlockList,
  useInteractiveResult,
  type SetOption,
} from "@/features/interactive-result"
import {
  isNoInput,
  isStagePlain,
  mergeOptions,
  OptionsForm,
  panelBlockTypes,
  panelPosition,
  panelWhen,
  schemaToFields,
  stageLayout,
  stageMode,
  visibleFields,
  type Field,
} from "@/features/tool-options"
import { baseName } from "@/shared/lib/base-name"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { filterSupported } from "../lib/filter-supported"
import { initialOptions } from "../lib/initial-options"
import { fillPatch, fillTargets, hideFilled } from "../lib/option-fill"
import { hiddenOptions, optionHints } from "../lib/option-hints"
import { panelState } from "../lib/panel-state"
import { showPanel } from "../lib/show-panel"
import { splitStageBlocks } from "../lib/split-stage-blocks"
import { usePanelWindow } from "../model/use-panel-window"
import { BottomPanel } from "./bottom-panel"
import { EditorStage } from "./editor-stage"

type Props = {
  tool: ToolEntry
  settings: Settings
  className?: string
}

/**
 * 파일을 만들지 않고 결과를 화면에 보여 주는 도구: 파일 하나 → 결과 블록 (복사·저장).
 * `x-input: "none"` 도구는 드롭 영역 없이 옵션만으로 바로 결과를 보여 준다.
 */
export function InteractiveToolWorkspace({ tool, settings, className }: Props) {
  const fields = useMemo(
    () => schemaToFields(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const noInput = useMemo(
    () => isNoInput(tool.optionsSchema),
    [tool.optionsSchema],
  )
  const [options, setOptions] = useState(() =>
    initialOptions(
      fields,
      settings.lastOptions[tool.id],
      settings.keepMetadata,
    ),
  )
  // 블록(리소스 목록·격자)이 옵션을 바꾸고 `resets`는 처음 값으로 (숨김 필드 기본값 포함)
  const defaults = useMemo(
    () => initialOptions(fields, undefined, settings.keepMetadata),
    [fields, settings.keepMetadata],
  )
  const setOption: SetOption = useCallback(
    (key, value, resets) =>
      setOptions((o) => applyOption(o, key, value, resets, defaults)),
    [defaults],
  )
  const [item, setItem] = useState<PathStat | null>(null)
  const [inputError, setInputError] = useState<string | null>(null)
  // 여러 줄 글(`x-multiline`)은 옵션 패널이 아니라 작업대의 편집기로 (`x-input: "none"` 도구만)
  const editors = noInput
    ? visibleFields(fields).filter(
        (f): f is Extract<Field, { kind: "multiline" }> =>
          f.kind === "multiline",
      )
    : []
  const panelFields = noInput
    ? visibleFields(fields).filter((f) => f.kind !== "multiline")
    : visibleFields(fields)
  // 옵션 패널 없이 작업대 위 막대에 옵션 (편집기 도구만)
  const layout = editors.length > 0 ? stageLayout(tool.optionsSchema) : null
  const merged = useMemo(() => mergeOptions(fields, options), [fields, options])
  const result = useInteractiveResult(
    tool.id,
    item?.path ?? null,
    merged,
    noInput,
  )
  // `x-fill-from`: 이 파일의 결과(이전 파일 결과 말고)가 오면 빈 칸을 결과 글로 채운다
  const fills = useMemo(() => fillTargets(fields), [fields])
  const fresh = result.data && !result.isPlaceholderData ? result.data : null
  useEffect(() => {
    if (!fresh) return
    const patch = fillPatch(fills, fresh, options)
    if (Object.keys(patch).length > 0) setOptions((o) => ({ ...o, ...patch }))
  }, [fresh, fills, options])

  // 한 파일만: 넣으면 바꿔치기, 폴더는 받지 않는다
  const add = (paths: string[]) => {
    if (paths.length === 0) return
    setInputError(null)
    statPaths(paths)
      .then((stats) => {
        const files = stats.filter((s) => !s.isDir)
        const first = filterSupported(files, tool.extensions).ok[0]
        if (!first) {
          toast.error(
            files.length === 0
              ? "이 도구는 폴더가 아닌 파일 하나를 받습니다."
              : "지원하지 않는 형식입니다.",
          )
          return
        }
        if (paths.length > 1) toast.info("한 번에 한 파일만 처리합니다.")
        // 새 파일은 채울 칸을 비워 그 파일의 결과로 다시 채운다
        if (first.path !== item?.path)
          setOptions((o) => ({
            ...o,
            ...Object.fromEntries(fills.map(([key]) => [key, ""])),
          }))
        setItem(first)
      })
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  const error =
    inputError ?? (result.isError ? toUserMessage(result.error) : null)
  // `optionHints` 블록: 값이 null인 옵션 칸에 보여 줄 값 (그리지 않고 옵션에도 넣지 않는다)
  const hints = useMemo(() => optionHints(result.data), [result.data])
  // 지금 결과에서 바꿀 수 없는 옵션(예: 선이 없는 아이콘의 두께)은 패널에서 감춘다
  const hidden = useMemo(() => hiddenOptions(result.data), [result.data])
  const shownPanelFields = useMemo(
    () => panelFields.filter((f) => !hidden.includes(f.key)),
    [panelFields, hidden],
  )

  // `x-panel-blocks`(`x-result-panel`은 코드만)는 옵션 패널 아래, `x-stage: "list-detail"`이면 리소스 목록은 왼쪽 열
  const mode = stageMode(tool.optionsSchema)
  const { list, stage, panel } = splitStageBlocks(
    result.data ? hideFilled(result.data, fills) : [],
    mode,
    panelBlockTypes(tool.optionsSchema),
  )
  // `x-panel-when`: 그 옵션이 비면 옵션 패널을 그리지 않는다
  const when = panelWhen(tool.optionsSchema)
  const panelShown = showPanel(when, options, !!layout)
  // `x-panel-position`: "bottom"은 오른쪽 패널 대신 작업대 아래에서 올라오는 패널, "window"는 별도 창
  const position = panelPosition(tool.optionsSchema)
  const bottom = position === "bottom"
  const asideShown = panelShown && position === "right"
  const closePanel = useCallback(() => {
    if (when) setOption(when, "")
  }, [when, setOption])
  usePanelWindow(
    panelShown && position === "window"
      ? panelState({
          title: tool.name,
          fields: panelFields,
          options,
          blocks: panel,
          data: result.data,
          fetching: result.isFetching,
          error,
        })
      : null,
    setOption,
    closePanel,
  )
  // 아래 패널이 덮는 높이: 결과 격자가 이만큼 더 스크롤되어 가려진 마지막 줄을 꺼낼 수 있게 (별도 창이면 0)
  const [inset, setInset] = useState(0)
  const bottomPanel = panelShown && bottom && (
    <BottomPanel
      fields={shownPanelFields}
      options={options}
      onChange={setOptions}
      blocks={panel}
      hints={hints}
      fetching={result.isFetching}
      error={error}
      onClose={when ? closePanel : undefined}
      onHeight={setInset}
    />
  )
  // 리소스 명령은 설치된 추가 도구만
  const addonId =
    tool.status === "installed" || tool.status === "updateAvailable"
      ? tool.id
      : undefined
  const fading = cn("transition-opacity", result.isFetching && "opacity-60")

  const results = result.data ? (
    <BlockList
      blocks={stage}
      toolId={addonId}
      values={options}
      onSetOption={setOption}
      className={cn("flex-1", fading)}
    />
  ) : (
    result.isFetching && <p className="text-stage-muted m-auto">만드는 중…</p>
  )

  return (
    <div className={cn("flex min-w-0 flex-1 flex-col", className)}>
      <div className="flex min-h-0 flex-1">
        <section
          aria-label="작업대"
          className={cn(
            "bg-stage text-stage-foreground my-2 ml-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl",
            // 글 편집기 도구·`x-stage-plain`은 어두운 미리보기 바탕 대신 테마 바탕
            (editors.length > 0 || isStagePlain(tool.optionsSchema)) &&
              "stage-plain",
            !asideShown && "mr-2",
            bottom && "relative",
          )}
          style={{ "--stage-inset": `${inset}px` } as CSSProperties}
        >
          {noInput && mode === "list-detail" ? (
            <div className="flex min-h-0 flex-1">
              <div className="flex w-64 shrink-0 flex-col border-r p-3">
                {list.length > 0 && (
                  <BlockList
                    blocks={list}
                    toolId={addonId}
                    values={options}
                    onSetOption={setOption}
                    className="flex-1"
                  />
                )}
              </div>
              <div
                className={cn(
                  "flex min-w-0 flex-1 flex-col gap-3 p-4",
                  bottom && "relative",
                )}
              >
                {result.data && stage.length === 0 ? (
                  <p className="text-stage-muted m-auto text-sm">
                    왼쪽 목록에서 고르세요.
                  </p>
                ) : (
                  results
                )}
                {/* 옵션 패널이 없을 때는 오류를 작업대에 */}
                {!panelShown && error && (
                  <ErrorBanner message={error} className="rounded-lg border" />
                )}
                {bottomPanel}
              </div>
            </div>
          ) : noInput && editors.length > 0 ? (
            <EditorStage
              tool={tool}
              fields={shownPanelFields}
              editors={editors}
              options={options}
              setOptions={setOptions}
              layout={layout}
              blocks={result.data}
              fetching={result.isFetching}
              error={error}
              onReset={() =>
                setOptions(
                  initialOptions(fields, undefined, settings.keepMetadata),
                )
              }
            />
          ) : noInput ? (
            <div className="flex min-h-0 flex-1 flex-col p-4">{results}</div>
          ) : item === null ? (
            <DropZone
              onAdd={add}
              onError={setInputError}
              single
              extensions={tool.extensions}
            />
          ) : (
            <>
              <div className="flex shrink-0 items-center gap-3 p-4">
                <Thumb item={item} className="size-16 rounded-md" />
                <span className="min-w-0 flex-1 truncate" title={item.path}>
                  {baseName(item.path)}
                </span>
                <DropZone
                  variant="tile"
                  single
                  extensions={tool.extensions}
                  onAdd={add}
                  onError={setInputError}
                />
              </div>
              <div className="flex min-h-0 flex-1 flex-col px-4 pb-4">
                {results}
              </div>
            </>
          )}
          {!(noInput && mode === "list-detail") && bottomPanel}
        </section>

        {asideShown && (
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
                  fields={shownPanelFields}
                  value={options}
                  hints={hints}
                  onChange={setOptions}
                />
              </div>
              {panel.length > 0 && (
                <BlockList
                  blocks={panel}
                  // 작업대 색 토큰을 테마 색으로 (밝은 패널 위). 스크롤 패널 안에서 줄지 않고 제 높이
                  className={cn("stage-plain shrink-0", fading)}
                />
              )}
              {error && (
                <ErrorBanner message={error} className="rounded-lg border" />
              )}
            </div>
          </aside>
        )}
      </div>

      {/* 실행할 것은 없지만 하단 막대 자리는 파일 도구(RunBar)와 같은 높이로 둔다.
          목록·상세 작업대(`x-stage: "list-detail"`)는 이 자리를 비워 두지 않고 격자를 아래까지 채운다 */}
      {!(noInput && mode === "list-detail") && (
        <div aria-hidden className="border-t py-3">
          <div className="h-9" />
        </div>
      )}
    </div>
  )
}
