import { cn } from "@/shared/lib/utils"
import { Checkbox } from "@/shared/ui/checkbox"
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
  const label = (
    <label htmlFor={id} className="text-muted-foreground min-w-0 text-sm">
      {field.label}
    </label>
  )
  const help =
    "description" in field && field.description ? (
      <p className="text-muted-foreground col-span-2 text-xs">
        {field.description}
      </p>
    ) : null
  // 이름 왼쪽, 입력 오른쪽 한 줄 (설명은 아래 줄)
  const row = cn(
    "grid grid-cols-[1fr_auto] items-center gap-x-3 gap-y-1",
    className,
  )

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
          </div>
          {help && <div className="pl-6">{help}</div>}
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
          {help}
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
          {help}
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
          {help}
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
          {help}
        </div>
      )
    case "text":
      return (
        <div className={row}>
          {label}
          <Input
            id={id}
            className="w-40"
            value={String(value ?? "")}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
          {help}
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
