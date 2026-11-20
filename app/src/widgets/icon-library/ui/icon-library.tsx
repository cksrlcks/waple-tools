import { useMemo } from "react"

import { RECOMMENDED, stylesOf } from "@/entities/icon-set"
import { SetDownloadPanel } from "@/features/icon-set-manage"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"

import { filterIcons } from "../lib/filter-icons"
import { useIconLibraryState } from "../model/use-icon-library-state"
import { IconDetailPanel } from "./icon-detail-panel"
import { IconGrid } from "./icon-grid"
import { IconToolbar } from "./icon-toolbar"

type Props = {
  className?: string
}

/** 받은 아이콘 세트를 격자로 보고, 고른 아이콘을 오른쪽 패널에서 복사·저장 (네트워크는 버튼을 누를 때만) */
export function IconLibrary({ className }: Props) {
  const s = useIconLibraryState()
  const { set, style, query, selected } = s
  const icons = useMemo(
    () =>
      set
        ? filterIcons(set.icons, { query, style, styles: stylesOf(set) })
        : null,
    [set, query, style],
  )
  const recommended = RECOMMENDED.find((r) => r.prefix === s.prefix)

  const body = !s.prefix ? null : !s.isInstalled ? (
    recommended && (
      <SetDownloadPanel
        prefix={recommended.prefix}
        name={recommended.name}
        license={recommended.license}
        className="m-auto"
      />
    )
  ) : s.iconSet.isError ? (
    <p className="text-destructive m-auto text-sm">
      {toUserMessage(s.iconSet.error)}
    </p>
  ) : set && icons ? (
    <IconGrid
      // 세트·스타일·검색이 바뀌면 처음부터 (스크롤·묶음 수 초기화)
      key={`${set.prefix}|${style}|${query}`}
      icons={icons}
      set={set}
      selectedName={selected?.name ?? null}
      onSelect={s.setSelected}
    />
  ) : (
    <p className="text-stage-muted m-auto text-sm">불러오는 중…</p>
  )

  return (
    <div className={cn("flex min-w-0 flex-1", className)}>
      <div className={cn("flex min-w-0 flex-1 flex-col", !selected && "mr-2")}>
        {s.installed && s.prefix && (
          <IconToolbar
            className="pt-2 pl-2"
            installed={s.installed}
            prefix={s.prefix}
            onPrefixChange={s.setPrefix}
            set={set}
            style={style}
            onStyleChange={s.setStyle}
            query={query}
            onQueryChange={s.setQuery}
            count={icons?.length ?? null}
          />
        )}
        <section
          aria-label="아이콘"
          className={cn(
            "bg-stage text-stage-foreground my-2 ml-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl",
            // 받기 안내는 테마 글자색을 쓰므로 밝은 바탕으로
            !s.isInstalled && "stage-plain",
          )}
        >
          {body}
        </section>
      </div>
      {selected && set && (
        <IconDetailPanel
          icon={selected}
          set={set}
          size={s.size}
          onSizeChange={s.setSize}
          stroke={s.stroke}
          onStrokeChange={s.setStroke}
          color={s.color}
          onColorChange={s.setColor}
        />
      )}
    </div>
  )
}
