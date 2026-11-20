import {
  iconSize,
  strokeOf,
  toSvg,
  type IconItem,
  type IconSet,
} from "@/entities/icon-set"
import { cn } from "@/shared/lib/utils"
import { Checkbox } from "@/shared/ui/checkbox"
import { Input } from "@/shared/ui/input"
import { Slider } from "@/shared/ui/slider"

import { scopeIds } from "../lib/scope-ids"
import { CopyActions } from "./copy-actions"

type Props = {
  icon: IconItem
  set: IconSet
  size: number
  onSizeChange: (size: number) => void
  stroke: number | null
  onStrokeChange: (stroke: number) => void
  color: string
  onColorChange: (color: string) => void
  className?: string
}

const row = "grid grid-cols-[1fr_auto] items-center gap-x-3"

/** 고른 아이콘: 미리보기 · 크기·두께·색 · 복사 */
export function IconDetailPanel({
  icon,
  set,
  size,
  onSizeChange,
  stroke,
  onStrokeChange,
  color,
  onColorChange,
  className,
}: Props) {
  // 입력 중인 빈 칸·범위 밖 값은 그릴 때만 8~1024로
  const px = Math.min(1024, Math.max(8, Math.round(size) || 8))
  const hasStroke = strokeOf(icon.body) !== null
  const opts = {
    size: px,
    stroke: hasStroke ? stroke : null,
    color: set.palette ? "currentColor" : color,
  }
  const { width, height } = iconSize(icon, set)
  const current = color === "currentColor"

  return (
    <aside
      aria-label="아이콘 정보"
      className={cn(
        "flex w-80 shrink-0 scrollbar-gutter-stable flex-col gap-5 overflow-y-auto px-6 pt-4 pb-5",
        className,
      )}
    >
      <div
        className="bg-muted flex h-40 shrink-0 items-center justify-center overflow-hidden rounded-lg p-4 [&>svg]:max-h-full [&>svg]:max-w-full"
        // body는 Rust에서 정리된 것만 들어온다 (app/src-tauri/src/addons/icons/sanitize.rs)
        dangerouslySetInnerHTML={{
          __html: scopeIds(toSvg(icon, set, opts), "preview"),
        }}
      />
      <header className="min-w-0">
        <h1 className="text-base font-semibold break-all">{icon.name}</h1>
        <p className="text-muted-foreground mt-1 text-xs">
          {set.name} · {width}×{height}
        </p>
      </header>
      <div className="flex flex-col gap-3">
        <div className={row}>
          <label htmlFor="icon-size" className="text-muted-foreground text-sm">
            크기
          </label>
          <span className="flex items-center gap-1.5">
            <Input
              id="icon-size"
              type="number"
              min={8}
              max={1024}
              className="w-20 text-right tabular-nums"
              value={size || ""}
              onChange={(e) => onSizeChange(Number(e.target.value))}
              onBlur={() => onSizeChange(px)}
            />
            <span className="text-muted-foreground text-sm">px</span>
          </span>
        </div>
        {hasStroke && stroke !== null && (
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground text-sm">두께</span>
              <span className="bg-muted min-w-9 rounded px-1.5 py-0.5 text-center text-xs font-medium tabular-nums">
                {stroke}
              </span>
            </div>
            <Slider
              min={0.5}
              max={3}
              step={0.25}
              value={[stroke]}
              onValueChange={(v) => onStrokeChange(Array.isArray(v) ? v[0] : v)}
              thumbLabel="두께"
            />
          </div>
        )}
        {!set.palette && (
          <div className={row}>
            <span className="text-muted-foreground text-sm">색</span>
            <span className="flex items-center gap-2">
              <label className="flex items-center gap-1.5 text-sm">
                <Checkbox
                  checked={current}
                  onCheckedChange={(c) =>
                    onColorChange(c ? "currentColor" : "#000000")
                  }
                />
                currentColor
              </label>
              <Input
                type="color"
                aria-label="색 고르기"
                className="w-12 p-0.5"
                value={current ? "#000000" : color}
                onChange={(e) => onColorChange(e.target.value)}
              />
            </span>
          </div>
        )}
      </div>
      <CopyActions icon={icon} set={set} opts={opts} />
    </aside>
  )
}
