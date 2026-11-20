import { useCallback } from "react"

import type { GridCell, GridChips, GridMore, GridSearch } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"
import { Segmented } from "@/shared/ui/segmented"

import type { SetOption } from "../model/set-option"
import { IconGridCell } from "./icon-grid-cell"

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
  more?: GridMore
  total: number
  onSetOption: SetOption
  className?: string
}

/** 격자: 위 줄 칩·검색·개수, 아래 카드 격자(폭에 맞춰 열 수가 바뀜), 끝에 [더 보기] */
export function IconGridBlock({
  cells,
  option,
  selected,
  chips,
  chip,
  search,
  query,
  more,
  total,
  onSetOption,
  className,
}: Props) {
  const select = useCallback(
    (key: string, was: boolean) => onSetOption(option, was ? "" : key),
    [onSetOption, option],
  )
  return (
    <div className={cn("flex min-h-0 flex-col gap-3", className)}>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
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
            className="w-56"
          />
        )}
        <span className="text-muted-foreground text-xs tabular-nums">
          {total.toLocaleString()}개
        </span>
      </div>
      {/* 아래 패널이 덮은 만큼(`--stage-inset`) 더 스크롤되게 */}
      <div className="-mx-1 min-h-0 flex-1 overflow-y-auto px-1 pt-1 pb-[calc(var(--stage-inset,0px)+0.5rem)]">
        {cells.length === 0 ? (
          <p className="text-muted-foreground py-10 text-center text-sm">
            찾는 항목이 없습니다.
          </p>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(5.5rem,1fr))] gap-x-3 gap-y-4">
            {cells.map((c) => (
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
        )}
        {more && (
          <div className="flex justify-center pt-4">
            <Button
              variant="outline"
              onClick={() => onSetOption(more.option, more.value)}
            >
              {more.label}
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
