import { IconCopy } from "@tabler/icons-react"
import { toast } from "sonner"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Checkbox } from "@/shared/ui/checkbox"
import { HelpTip } from "@/shared/ui/help-tip"
import { Input } from "@/shared/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select"
import { Slider } from "@/shared/ui/slider"

import type { Field } from "../lib/schema-to-fields"
import { sliderDisplay, sliderTick } from "../lib/slider-display"
import { FileField } from "./file-field"
import { RegexField } from "./regex-field"
import { SliderInput } from "./slider-input"
import { SwatchField } from "./swatch-field"

type Props = {
  field: Field
  value: unknown
  /** 값이 null일 때 보여 줄 값 (예: 원래 두께). null 허용 슬라이더만 쓴다 */
  hint?: unknown
  onChange: (value: unknown) => void
  disabled?: boolean
  className?: string
}

export function OptionField({
  field,
  value,
  hint,
  onChange,
  disabled,
  className,
}: Props) {
  const id = `opt-${field.key}`
  const labelId = `${id}-label`
  // 설명은 라벨 옆 물음표 풍선으로 (아래 줄에 깔면 패널이 길어진다)
  const tip =
    "description" in field && field.description ? (
      <HelpTip text={field.description} />
    ) : null
  const label = (
    <span className="flex min-w-0 items-center gap-1">
      <label
        id={labelId}
        htmlFor={id}
        className="text-muted-foreground min-w-0 text-sm"
      >
        {field.label}
      </label>
      {tip}
    </span>
  )
  // 이름 왼쪽, 입력 오른쪽 한 줄
  const row = cn("grid grid-cols-[1fr_auto] items-center gap-x-3", className)

  switch (field.kind) {
    case "checkbox":
      return (
        <div className={cn("flex flex-col gap-1", className)}>
          <div className="flex items-center gap-2">
            <Checkbox
              id={id}
              checked={value === true}
              onCheckedChange={(c) => onChange(c === true)}
              disabled={disabled}
            />
            <label htmlFor={id} className="text-sm">
              {field.label}
            </label>
            {tip}
          </div>
        </div>
      )
    case "slider": {
      // null이면 힌트 자리에 손잡이 + "2 · 원래" (힌트는 옵션에 쓰지 않는다)
      const shown = sliderDisplay(value, field, hint)
      // 원래 값 눈금: 손잡이 가운데와 맞춘다 (thumbAlignment="edge", 손잡이 size-3)
      const tick = field.nullable ? sliderTick(field, hint) : undefined
      return (
        <div className={cn("flex flex-col gap-2", className)}>
          <div className="flex items-center justify-between">
            {label}
            {/* 입력칸이 있으면 값은 입력칸이 보여 준다 */}
            {!field.input && (
              <span className="bg-muted min-w-9 rounded px-1.5 py-0.5 text-center text-xs font-medium tabular-nums">
                {shown.badge}
              </span>
            )}
          </div>
          <div className="flex items-center gap-3">
            <div className="relative flex min-w-0 flex-1 items-center">
              {tick !== undefined && (
                <span
                  aria-hidden
                  className="bg-muted-foreground pointer-events-none absolute top-1/2 h-2.5 w-px -translate-x-1/2 -translate-y-1/2"
                  style={{
                    left: `calc(0.375rem + (100% - 0.75rem) * ${tick})`,
                  }}
                />
              )}
              <Slider
                id={id}
                min={field.min}
                max={field.max}
                step={field.step}
                value={[shown.thumb]}
                onValueChange={(v) => onChange(Array.isArray(v) ? v[0] : v)}
                disabled={disabled}
              />
            </div>
            {field.input && (
              <SliderInput
                value={value}
                field={field}
                hint={hint}
                labelId={labelId}
                onChange={onChange}
                disabled={disabled}
              />
            )}
            {field.nullable && (
              <Button
                size="sm"
                variant="outline"
                className="shrink-0"
                disabled={disabled || value === null}
                onClick={() => onChange(null)}
              >
                원래대로
              </Button>
            )}
          </div>
        </div>
      )
    }
    case "number":
      return (
        <div className={row}>
          {label}
          <Input
            id={id}
            type="number"
            className="w-24 text-right tabular-nums"
            min={field.min}
            max={field.max}
            step={field.step ?? (field.integer ? 1 : "any")}
            value={value === null || value === undefined ? "" : String(value)}
            onChange={(e) =>
              onChange(e.target.value === "" ? null : Number(e.target.value))
            }
            disabled={disabled}
          />
        </div>
      )
    case "select":
      return (
        <div className={row}>
          {label}
          <Select
            items={field.options}
            value={String(value)}
            onValueChange={(v) => onChange(v)}
            disabled={disabled}
          >
            <SelectTrigger id={id} className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {field.options.map((o) => (
                <SelectItem key={o.value} value={o.value}>
                  {o.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )
    case "color":
      return (
        <div className={row}>
          {label}
          <Input
            id={id}
            type="color"
            className="w-12 p-0.5"
            value={String(value)}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
        </div>
      )
    case "swatches":
      return (
        <div className={row}>
          {label}
          <SwatchField
            id={id}
            labelId={labelId}
            value={String(value ?? field.default)}
            swatches={field.swatches}
            onChange={onChange}
            disabled={disabled}
          />
        </div>
      )
    case "text":
      return (
        <div
          className={field.wide ? cn("flex flex-col gap-2", className) : row}
        >
          {label}
          <Input
            id={id}
            className={field.wide ? undefined : "w-40"}
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
        </div>
      )
    case "password":
      return (
        <div className={row}>
          {label}
          <Input
            id={id}
            type="password"
            autoComplete="off"
            className="w-40"
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
        </div>
      )
    case "file":
      return (
        <div className={row}>
          {label}
          <FileField
            id={id}
            label={field.label}
            value={String(value ?? "")}
            extensions={field.extensions}
            onChange={onChange}
            disabled={disabled}
          />
        </div>
      )
    // 입력 없는 도구는 작업대 편집기로 그려 여기 오지 않는다. 파일을 받는 도구는 패널 안 글상자
    case "multiline":
      return (
        <div className={cn("flex flex-col gap-2", className)}>
          <div className="flex items-center justify-between gap-2">
            {label}
            {field.copy && (
              <Button
                size="sm"
                variant="outline"
                disabled={!value}
                onClick={() =>
                  navigator.clipboard
                    .writeText(String(value ?? ""))
                    .then(() => toast.success("복사했습니다."))
                    .catch(() => toast.error("복사할 수 없습니다."))
                }
              >
                <IconCopy data-icon="inline-start" stroke={1.75} />
                복사
              </Button>
            )}
          </div>
          <textarea
            id={id}
            rows={8}
            spellCheck={false}
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            className="border-input bg-card placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-primary/40 dark:bg-input/30 min-h-24 w-full resize-y rounded-md border px-2 py-1.5 font-mono text-xs outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-50"
          />
        </div>
      )
    case "regex":
      return (
        <RegexField
          label={field.label}
          value={typeof value === "string" ? value : field.default}
          onChange={onChange}
          className={className}
        />
      )
    case "unsupported":
      return (
        <div className={cn("flex flex-col gap-1", className)}>
          <span className="text-sm">{field.label}</span>
          <p className="text-muted-foreground text-xs">
            이 옵션은 기본값을 사용합니다.
          </p>
        </div>
      )
  }
}
