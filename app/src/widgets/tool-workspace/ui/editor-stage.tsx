import type { Dispatch, SetStateAction } from "react"
import { IconTrash } from "@tabler/icons-react"

import type { Block, Span, ToolEntry } from "@/entities/tool"
import { TextEditor } from "@/features/file-input"
import { BlockList } from "@/features/interactive-result"
import { OptionsBar, RegexField, type Field } from "@/features/tool-options"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { ErrorBanner } from "@/shared/ui/error-banner"

import { editorMarks } from "../lib/editor-marks"

type Options = Record<string, unknown>

type Props = {
  tool: ToolEntry
  /** 편집기(`x-multiline`)가 아닌 옵션 */
  fields: Field[]
  editors: Extract<Field, { kind: "multiline" }>[]
  options: Options
  setOptions: Dispatch<SetStateAction<Options>>
  /** `x-layout`. null이면 옵션은 오른쪽 패널에 있다 */
  layout: "full" | "split" | null
  blocks: Block[] | undefined
  fetching: boolean
  error: string | null
  /** [지우기]: 옵션과 글을 처음 값으로 */
  onReset: () => void
  className?: string
}

/** 글 편집기 도구(`x-input: "none"` + `x-multiline`)의 작업대: 편집기 + 결과, `x-layout`이면 위에 옵션 막대 */
export function EditorStage({
  tool,
  fields,
  editors,
  options,
  setOptions,
  layout,
  blocks,
  fetching,
  error,
  onReset,
  className,
}: Props) {
  const regexFields = layout ? fields.filter((f) => f.kind === "regex") : []
  const barFields = fields.filter((f) => f.kind !== "regex")
  const flags = fields
    .map((f) =>
      f.kind === "checkbox" && f.short && options[f.key] === true
        ? f.short
        : "",
    )
    .join("")
  const values = Object.fromEntries(
    editors.map((f) => [f.key, String(options[f.key] ?? "")]),
  )
  // 편집기가 모두 비었으면 결과(입력 안내)는 숨기고 편집기를 꽉 채운다 (좌우 배치는 결과 칸이 늘 있다)
  const allEmpty = Object.values(values).every((v) => v.trim() === "")
  const { marks, rest } = blocks
    ? editorMarks(blocks, values)
    : { marks: {} as Record<string, Span[]>, rest: undefined }
  const set = (key: string) => (v: unknown) =>
    setOptions((o) => ({ ...o, [key]: v }))

  const results = rest ? (
    <BlockList
      blocks={rest}
      icon={false}
      className={cn("flex-1 transition-opacity", fetching && "opacity-60")}
    />
  ) : (
    fetching && <p className="text-stage-muted m-auto">만드는 중…</p>
  )

  const editorList = editors.map((f) => (
    <TextEditor
      key={f.key}
      label={f.label}
      placeholder={f.description}
      value={values[f.key] ?? ""}
      onChange={set(f.key)}
      marks={marks[f.key]}
    />
  ))

  return (
    <div className={cn("flex min-h-0 flex-1 flex-col gap-3 p-4", className)}>
      {layout && (
        <header className="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-2">
          <h1 className="text-base font-semibold">{tool.name}</h1>
          <OptionsBar
            fields={barFields}
            value={options}
            onChange={setOptions}
          />
          <Button variant="ghost" onClick={onReset} className="ml-auto">
            <IconTrash data-icon="inline-start" stroke={1.75} />
            지우기
          </Button>
        </header>
      )}
      {regexFields.map((f) => (
        <RegexField
          key={f.key}
          label={f.label}
          value={String(options[f.key] ?? "")}
          onChange={set(f.key)}
          flags={flags}
          className="shrink-0"
        />
      ))}
      {layout && error && (
        <ErrorBanner message={error} className="rounded-lg border" />
      )}
      {layout === "split" ? (
        <div className="grid min-h-0 flex-1 grid-cols-2 gap-3">
          <div className="flex min-h-0 flex-col gap-3 *:flex-1">
            {editorList}
          </div>
          <section
            aria-label="결과"
            className="flex min-h-0 flex-col rounded-lg border p-3"
          >
            {results}
          </section>
        </div>
      ) : (
        <>
          <div className="grid min-h-0 flex-1 auto-cols-fr grid-flow-col gap-3">
            {editorList}
          </div>
          {!allEmpty && results}
        </>
      )}
    </div>
  )
}
