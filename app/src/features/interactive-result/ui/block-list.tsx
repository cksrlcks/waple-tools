import { IconWorld } from "@tabler/icons-react"

import type { Block } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { formatNumber } from "../lib/clip-text"
import type { SetOption } from "../model/set-option"
import { CodeBlock } from "./code-block"
import { ColorGrid } from "./color-grid"
import { DiffBlock } from "./diff-block"
import { HighlightBlock } from "./highlight-block"
import { IconGridBlock } from "./icon-grid-block"
import { ImageBlock } from "./image-block"
import { RecordsBlock } from "./records-block"
import { ResourceListBlock } from "./resource-list-block"
import { TableBlock } from "./table-block"

type Props = {
  blocks: Block[]
  /** 안내 글만 있을 때 위에 아이콘을 둔다 (기본 켬) */
  icon?: boolean
  /** 설치된 추가 도구 id: 있을 때만 리소스 목록(받기·삭제)을 그린다 */
  toolId?: string
  /** 지금 옵션 값: 블록이 바꾼 값을 결과가 오기 전에도 보여 준다 (검색 입력이 끊기지 않게) */
  values?: Record<string, unknown>
  /** 있을 때만 옵션을 바꾸는 블록(리소스 목록·격자)을 그린다 */
  onSetOption?: SetOption
  className?: string
}

/**
 * 결과 블록을 차례로 그린다: 리소스 목록 → 안내 글 → 격자 → 이미지 → 숫자 → 색 → 강조 글 → 비교 → 카드 → 표 → 코드.
 * 강조 글·비교·코드는 남는 높이를 나눠 갖고(flex-1) 안에서 스크롤, 표는 제 높이만큼(넘치면 줄어든다).
 */
// ponytail: imagePair 블록은 아직 쓰는 도구가 없어 그리지 않는다
export function BlockList({
  blocks,
  icon = true,
  toolId,
  values,
  onSetOption,
  className,
}: Props) {
  // 블록이 바꾸는 옵션의 지금 값 (옵션에 없으면 블록이 알려 준 값)
  const current = (key: string, fallback?: string) => {
    const v = values?.[key]
    return typeof v === "string" ? v : fallback
  }
  const lists = blocks.filter((b) => b.type === "resourceList")
  const grids = blocks.filter((b) => b.type === "iconGrid")
  const texts = blocks.filter((b) => b.type === "text")
  const images = blocks.filter((b) => b.type === "image")
  const numbers = blocks.filter((b) => b.type === "number")
  const colors = blocks.filter((b) => b.type === "color")
  const highlights = blocks.filter((b) => b.type === "highlight")
  const diffs = blocks.filter((b) => b.type === "diff")
  const records = blocks.filter((b) => b.type === "records")
  const tables = blocks.filter((b) => b.type === "table")
  const codes = blocks.filter((b) => b.type === "code")
  // 안내 글만 있으면(입력 대기 등) 빈 작업대처럼 가운데에 둔다
  const onlyText = texts.length === blocks.length
  return (
    <div
      className={cn(
        "flex min-h-0 flex-col gap-3",
        onlyText && "items-center justify-center",
        className,
      )}
    >
      {onlyText && icon && (
        <span className="bg-stage-foreground/8 text-stage-muted flex size-14 items-center justify-center rounded-2xl">
          <IconWorld className="size-7" stroke={1.5} />
        </span>
      )}
      {toolId &&
        onSetOption &&
        lists.map((b, i) => (
          <ResourceListBlock
            key={`l${i}`}
            toolId={toolId}
            items={b.items}
            selected={current(b.option, b.selected)}
            onSelect={(id) => onSetOption(b.option, id, b.resets)}
            className="flex-1"
          />
        ))}
      {texts.map((b, i) => (
        <p
          key={`t${i}`}
          className={
            onlyText
              ? "text-stage-muted text-sm"
              : "bg-stage-foreground/8 rounded-lg px-3 py-2 text-xs"
          }
        >
          {b.text}
        </p>
      ))}
      {onSetOption &&
        grids.map((b, i) => {
          const chip = current(b.chips?.option ?? "")
          return (
            <IconGridBlock
              key={`g${i}`}
              cells={b.cells}
              option={b.option}
              selected={current(b.option, b.selected)}
              chips={b.chips}
              chip={
                b.chips?.choices.some((c) => c.value === chip)
                  ? (chip ?? "")
                  : (b.chips?.value ?? "")
              }
              search={b.search}
              query={current(b.search?.option ?? "", b.search?.value) ?? ""}
              more={b.more}
              total={b.total}
              onSetOption={onSetOption}
              className="flex-1"
            />
          )
        })}
      {images.map((b, i) => (
        <ImageBlock key={`i${i}`} src={b.src} name={b.name} />
      ))}
      {numbers.length > 0 && (
        <dl className="flex flex-wrap gap-x-6 gap-y-1 text-xs">
          {numbers.map((b, i) => (
            <div key={`n${i}`} className="flex gap-2">
              <dt className="text-stage-muted">{b.label}</dt>
              <dd className="tabular-nums">{formatNumber(b.value, b.unit)}</dd>
            </div>
          ))}
        </dl>
      )}
      {colors.length > 0 && <ColorGrid colors={colors} />}
      {highlights.map((b, i) => (
        <HighlightBlock
          key={`h${i}`}
          spans={b.spans}
          className="min-h-28 flex-1"
        />
      ))}
      {diffs.map((b, i) => (
        <DiffBlock
          key={`d${i}`}
          layout={b.layout}
          lines={b.lines}
          className="min-h-28 flex-1"
        />
      ))}
      {records.map((b, i) => (
        <RecordsBlock
          key={`c${i}`}
          items={b.items}
          copy={b.copy}
          className="flex-1"
        />
      ))}
      {tables.map((b, i) => (
        <TableBlock
          key={`r${i}`}
          columns={b.columns}
          rows={b.rows}
          name={b.name}
        />
      ))}
      {codes.map((b, i) => (
        <CodeBlock
          key={`k${i}`}
          code={b.code}
          lang={b.lang}
          name={b.name}
          className="flex-1"
        />
      ))}
    </div>
  )
}
