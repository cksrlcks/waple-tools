import { cn } from "@/shared/lib/utils"
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

type Props = {
  field: Field
  value: unknown
  onChange: (value: unknown) => void
  disabled?: boolean
  className?: string
}

export function OptionField({
  field,
  value,
  onChange,
  disabled,
  className,
}: Props) {
  const id = `opt-${field.key}`
  // 설명은 라벨 옆 물음표 풍선으로 (아래 줄에 깔면 패널이 길어진다)
  const tip =
    "description" in field && field.description ? (
      <HelpTip text={field.description} />
    ) : null
  const label = (
    <span className="flex min-w-0 items-center gap-1">
      <label htmlFor={id} className="text-muted-foreground min-w-0 text-sm">
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
    case "slider":
      return (
        <div className={cn("flex flex-col gap-2", className)}>
          <div className="flex items-center justify-between">
            {label}
            <span className="bg-muted min-w-9 rounded px-1.5 py-0.5 text-center text-xs font-medium tabular-nums">
              {String(value)}
            </span>
          </div>
          <Slider
            id={id}
            min={field.min}
            max={field.max}
            step={field.step}
            value={[Number(value)]}
            onValueChange={(v) => onChange(Array.isArray(v) ? v[0] : v)}
            disabled={disabled}
          />
        </div>
      )
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
            step={field.integer ? 1 : "any"}
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
