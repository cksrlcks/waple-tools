import { useEffect, useMemo, useState } from "react"
import { toast } from "sonner"

import type { Settings } from "@/entities/settings"
import type { PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, statPaths } from "@/features/file-input"
import { Thumb } from "@/features/image-preview"
import { BlockList, useInteractiveResult } from "@/features/interactive-result"
import {
  isNoInput,
  mergeOptions,
  OptionsForm,
  resultPanel,
  schemaToFields,
  stageLayout,
  type Field,
} from "@/features/tool-options"
import { baseName } from "@/shared/lib/base-name"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { filterSupported } from "../lib/filter-supported"
import { initialOptions } from "../lib/initial-options"
import { fillPatch, fillTargets, hideFilled } from "../lib/option-fill"
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
  const [item, setItem] = useState<PathStat | null>(null)
  const [inputError, setInputError] = useState<string | null>(null)
  // 여러 줄 글(`x-multiline`)은 옵션 패널이 아니라 작업대의 편집기로 (`x-input: "none"` 도구만)
  const editors = noInput
    ? fields.filter(
        (f): f is Extract<Field, { kind: "multiline" }> =>
          f.kind === "multiline",
      )
    : []
  const panelFields = noInput
    ? fields.filter((f) => f.kind !== "multiline")
    : fields
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

  // `x-result-panel`: 코드 블록은 옵션 패널 아래, 나머지는 작업대
  const shown = result.data ? hideFilled(result.data, fills) : []
  const toPanel = resultPanel(tool.optionsSchema)
  const panelCodes = toPanel ? shown.filter((b) => b.type === "code") : []

  const results = result.data ? (
    <BlockList
      blocks={toPanel ? shown.filter((b) => b.type !== "code") : shown}
      className={cn(
        "flex-1 transition-opacity",
        result.isFetching && "opacity-60",
      )}
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
            // 글 편집기 도구는 어두운 미리보기 바탕 대신 테마 바탕
            editors.length > 0 && "stage-plain",
            layout && "mr-2",
          )}
        >
          {noInput && editors.length > 0 ? (
            <EditorStage
              tool={tool}
              fields={panelFields}
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
        </section>

        {!layout && (
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
                  fields={panelFields}
                  value={options}
                  onChange={setOptions}
                />
              </div>
              {panelCodes.length > 0 && (
                <BlockList
                  blocks={panelCodes}
                  className={cn(
                    // 작업대 색 토큰을 테마 색으로 (밝은 패널 위). 스크롤 패널 안에서 줄지 않고 제 높이
                    "stage-plain shrink-0 transition-opacity",
                    result.isFetching && "opacity-60",
                  )}
                />
              )}
              {error && (
                <ErrorBanner message={error} className="rounded-lg border" />
              )}
            </div>
          </aside>
        )}
      </div>

      {/* 실행할 것은 없지만 하단 막대 자리는 파일 도구(RunBar)와 같은 높이로 둔다 */}
      <div aria-hidden className="border-t py-3">
        <div className="h-9" />
      </div>
    </div>
  )
}
