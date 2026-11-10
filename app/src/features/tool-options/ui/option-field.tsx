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
    <label htmlFor={id} className="text-sm">
      {field.label}
    </label>
  )
  const help =
    "description" in field && field.description ? (
      <p className="text-muted-foreground text-xs">{field.description}</p>
    ) : null

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
            {label}
          </div>
          {help}
        </div>
      )
    case "slider":
      return (
        <div className={cn("flex flex-col gap-1.5", className)}>
          <div className="flex items-center justify-between">
            {label}
            <span className="text-muted-foreground text-xs tabular-nums">
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
        <div className={cn("flex flex-col gap-1", className)}>
          {label}
          <Input
            id={id}
            type="number"
            className="w-32"
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
        <div className={cn("flex flex-col gap-1", className)}>
          {label}
          <Select
            items={field.options}
            value={String(value)}
            onValueChange={(v) => onChange(v)}
            disabled={disabled}
          >
            <SelectTrigger id={id} className="min-w-40">
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
        <div className={cn("flex flex-col gap-1", className)}>
          {label}
          <Input
            id={id}
            type="color"
            className="w-16 p-0.5"
            value={String(value)}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
          {help}
        </div>
      )
    case "text":
      return (
        <div className={cn("flex flex-col gap-1", className)}>
          {label}
          <Input
            id={id}
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
