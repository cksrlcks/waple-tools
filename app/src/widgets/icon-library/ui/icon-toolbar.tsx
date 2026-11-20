import { useState } from "react"

import {
  RECOMMENDED,
  stylesOf,
  type IconSet,
  type InstalledSet,
} from "@/entities/icon-set"
import {
  CollectionsDialog,
  SetActionsMenu,
  useCheckUpdates,
} from "@/features/icon-set-manage"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"
import { Segmented } from "@/shared/ui/segmented"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select"

type Props = {
  installed: InstalledSet[]
  prefix: string
  onPrefixChange: (prefix: string) => void
  /** 받은 세트를 읽은 뒤에만 */
  set?: IconSet
  style: string | null
  onStyleChange: (style: string) => void
  query: string
  onQueryChange: (query: string) => void
  /** 거른 아이콘 수 (세트를 읽기 전엔 null) */
  count: number | null
  className?: string
}

export function IconToolbar({
  installed,
  prefix,
  onPrefixChange,
  set,
  style,
  onStyleChange,
  query,
  onQueryChange,
  count,
  className,
}: Props) {
  const [moreOpen, setMoreOpen] = useState(false)
  const check = useCheckUpdates()
  const current = installed.find((s) => s.prefix === prefix)
  const items = [
    ...installed.map((s) => ({ value: s.prefix, label: s.name })),
    ...RECOMMENDED.filter(
      (r) => !installed.some((s) => s.prefix === r.prefix),
    ).map((r) => ({ value: r.prefix, label: `${r.name} (받기 전)` })),
  ]
  const styles = set ? stylesOf(set) : {}
  const styleOptions = Object.entries(styles).map(([value, label]) => ({
    value,
    label,
  }))

  return (
    <div className={cn("flex flex-wrap items-center gap-2", className)}>
      <Select
        items={items}
        value={prefix}
        onValueChange={(v) => {
          if (v) onPrefixChange(v)
        }}
      >
        <SelectTrigger className="w-48" aria-label="아이콘 세트">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {items.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {current && <SetActionsMenu set={current} />}
      {styleOptions.length > 1 && style !== null && (
        <Segmented
          options={styleOptions}
          value={style}
          onChange={onStyleChange}
          aria-label="스타일"
        />
      )}
      {current && (
        <Input
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          placeholder="아이콘 검색"
          aria-label="아이콘 검색"
          className="w-48"
        />
      )}
      {current && count !== null && (
        <span className="text-muted-foreground text-xs tabular-nums">
          {count.toLocaleString()}개 · v{current.version}
        </span>
      )}
      <div className="ml-auto flex items-center gap-2">
        <Button
          variant="outline"
          disabled={installed.length === 0 || check.isPending}
          onClick={() => check.mutate()}
        >
          {check.isPending ? "확인 중…" : "업데이트 확인"}
        </Button>
        <Button variant="outline" onClick={() => setMoreOpen(true)}>
          세트 더 보기
        </Button>
      </div>
      <CollectionsDialog
        open={moreOpen}
        onOpenChange={setMoreOpen}
        installed={installed}
      />
    </div>
  )
}
