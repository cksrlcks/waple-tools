import { useMemo, useState } from "react"
import { toast } from "sonner"

import type { Settings } from "@/entities/settings"
import type { PathStat, ToolEntry } from "@/entities/tool"
import { DropZone, statPaths } from "@/features/file-input"
import { Thumb } from "@/features/image-preview"
import { BlockList, useInteractiveResult } from "@/features/interactive-result"
import {
  mergeOptions,
  OptionsForm,
  schemaToFields,
} from "@/features/tool-options"
import { baseName } from "@/shared/lib/base-name"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { filterSupported } from "../lib/filter-supported"
import { initialOptions } from "../lib/initial-options"

type Props = {
  tool: ToolEntry
  settings: Settings
  className?: string
}

/** 파일을 만들지 않고 결과를 화면에 보여 주는 도구: 파일 하나 → 결과 블록 (복사) */
export function InteractiveToolWorkspace({ tool, settings, className }: Props) {
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
  const [item, setItem] = useState<PathStat | null>(null)
  const [inputError, setInputError] = useState<string | null>(null)
  const result = useInteractiveResult(
    tool.id,
    item?.path ?? null,
    mergeOptions(fields, options),
  )

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
        setItem(first)
      })
      .catch(() => setInputError("파일 정보를 읽을 수 없습니다."))
  }

  const error =
    inputError ?? (result.isError ? toUserMessage(result.error) : null)

  return (
    <div className={cn("flex min-w-0 flex-1", className)}>
      <section
        aria-label="작업대"
        className="bg-stage text-stage-foreground my-2 ml-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl"
      >
        {item === null ? (
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
              {result.data ? (
                <BlockList
                  blocks={result.data}
                  className={cn(
                    "flex-1 transition-opacity",
                    result.isFetching && "opacity-60",
                  )}
                />
              ) : (
                result.isFetching && (
                  <p className="text-stage-muted m-auto">만드는 중…</p>
                )
              )}
            </div>
          </>
        )}
      </section>

      <aside aria-label="옵션" className="flex w-80 shrink-0 flex-col xl:w-88">
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
            />
          </div>
          {error && (
            <ErrorBanner message={error} className="rounded-lg border" />
          )}
        </div>
      </aside>
    </div>
  )
}
