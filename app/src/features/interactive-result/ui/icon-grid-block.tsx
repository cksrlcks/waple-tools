import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import type { GridCell, GridChips, GridSearch } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Input } from "@/shared/ui/input"
import { Segmented } from "@/shared/ui/segmented"

import { columnCount, revealScrollTop, visibleRange } from "../lib/grid-virtual"
import type { SetOption } from "../model/set-option"
import { IconGridCell } from "./icon-grid-cell"

// 카드 크기 (px): 최소 폭 5.5rem, 열 간격 gap-x-3, 줄 간격 gap-y-4, 이름 줄(gap-1.5 + text-xs 1rem)
const CARD_MIN = 88
const GAP_X = 12
const GAP_Y = 16
const LABEL = 22
const OVERSCAN = 3
// 스크롤 칸 아래 여백(+0.5rem). 위 여백(pt-1, 4px)은 계산에서 무시한다
const PAD_BOTTOM = 8

type Props = {
  cells: GridCell[]
  option: string
  /** 고른 칸 key (지금 옵션 값) */
  selected?: string
  chips?: GridChips
  /** 칩 값 (지금 옵션 값) */
  chip: string
  search?: GridSearch
  /** 검색어 (지금 옵션 값: 결과를 기다리는 동안에도 입력이 끊기지 않게) */
  query: string
  total: number
  /** 아래에서 격자를 덮는 패널 높이(px): 고른 칸을 그 위로 스크롤한다 */
  inset?: number
  onSetOption: SetOption
  className?: string
}

/**
 * 격자: 위 줄 왼쪽 개수·칩, 오른쪽 끝 검색. 아래 카드 격자(폭에 맞춰 열 수가 바뀜)는
 * 보이는 줄만 그린다 (카드 크기가 정해져 있어 줄 높이를 계산할 수 있다).
 */
export function IconGridBlock({
  cells,
  option,
  selected,
  chips,
  chip,
  search,
  query,
  total,
  inset = 0,
  onSetOption,
  className,
}: Props) {
  const select = useCallback(
    (key: string, was: boolean) => onSetOption(option, was ? "" : key),
    [onSetOption, option],
  )
  const scrollRef = useRef<HTMLDivElement>(null)
  const [box, setBox] = useState({ width: 0, height: 0 })
  const [scrollTop, setScrollTop] = useState(0)
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    const ro = new ResizeObserver(() =>
      // 좌우 여백(px-1) 뺀 폭
      setBox({ width: el.clientWidth - 8, height: el.clientHeight }),
    )
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const cols = columnCount(box.width, CARD_MIN, GAP_X)
  const cardW = Math.max(
    CARD_MIN,
    (box.width - GAP_X * (cols - 1)) / cols || CARD_MIN,
  )
  const rowHeight = cardW + LABEL + GAP_Y
  const rows = Math.ceil(cells.length / cols)
  const { start, end } = visibleRange({
    scrollTop,
    viewport: box.height,
    rowHeight,
    rows,
    overscan: OVERSCAN,
  })

  // 다른 세트·칩·검색어로 칸 목록이 바뀌면 맨 위로 (옵션만 바뀐 재실행은 같은 목록)
  const signature = useMemo(() => cells.map((c) => c.key).join("\n"), [cells])
  // 목록이 바뀐 렌더에서는 고른 칸으로 스크롤하지 않는다 (맨 위가 이긴다)
  const prevSignature = useRef(signature)
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 })
  }, [signature])

  // 칸을 고르거나 아래 패널이 열려 덮으면, 고른 칸 줄을 패널 위로
  const selectedIndex = useMemo(
    () => (selected ? cells.findIndex((c) => c.key === selected) : -1),
    [cells, selected],
  )
  useEffect(() => {
    const changed = prevSignature.current !== signature
    prevSignature.current = signature
    const el = scrollRef.current
    if (changed || !el || selectedIndex < 0 || box.height === 0) return
    const top = revealScrollTop({
      scrollTop: el.scrollTop,
      viewport: el.clientHeight,
      inset: inset + PAD_BOTTOM,
      rowHeight,
      row: Math.floor(selectedIndex / cols),
    })
    if (top !== el.scrollTop) el.scrollTo({ top, behavior: "smooth" })
  }, [signature, selectedIndex, inset, cols, rowHeight, box.height])

  return (
    <div className={cn("flex min-h-0 flex-col gap-3", className)}>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <span className="text-muted-foreground text-xs tabular-nums">
          {total.toLocaleString()}개
        </span>
        {chips && chips.choices.length > 1 && (
          <Segmented
            options={chips.choices}
            value={chip}
            onChange={(v) => onSetOption(chips.option, v, chips.resets)}
            aria-label="보기 방식"
          />
        )}
        {search && (
          <Input
            value={query}
            onChange={(e) =>
              onSetOption(search.option, e.target.value, search.resets)
            }
            placeholder={search.placeholder}
            aria-label={search.placeholder}
            className="ml-auto w-56"
          />
        )}
      </div>
      {/* 아래 패널이 덮은 만큼(`--stage-inset`) 더 스크롤되게 */}
      <div
        ref={scrollRef}
        onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
        className="scrollbar-gutter-stable -mx-1 min-h-0 flex-1 overflow-y-auto px-1 pt-1 pb-[calc(var(--stage-inset,0px)+0.5rem)]"
      >
        {cells.length === 0 ? (
          <p className="text-muted-foreground py-10 text-center text-sm">
            찾는 항목이 없습니다.
          </p>
        ) : (
          <div className="relative" style={{ height: rows * rowHeight }}>
            {Array.from({ length: end - start }, (_, i) => {
              const r = start + i
              return (
                <div
                  key={r}
                  className="absolute inset-x-0 grid gap-x-3"
                  style={{
                    top: r * rowHeight,
                    height: rowHeight - GAP_Y,
                    gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`,
                  }}
                >
                  {cells.slice(r * cols, (r + 1) * cols).map((c) => (
                    <IconGridCell
                      key={c.key}
                      cellKey={c.key}
                      svg={c.svg}
                      title={c.title}
                      selected={c.key === selected}
                      onSelect={select}
                    />
                  ))}
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
