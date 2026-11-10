import { useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"

import type { OutputPolicy } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Checkbox } from "@/shared/ui/checkbox"
import { Input } from "@/shared/ui/input"
import { RadioGroup, RadioGroupItem } from "@/shared/ui/radio-group"

type Props = {
  value: OutputPolicy
  onChange: (policy: OutputPolicy) => void
  /** "원본 옆에 저장"을 처음 고를 때의 접미사 (예: `_webp`) */
  defaultSuffix: string
  recursive: boolean
  onRecursiveChange: (recursive: boolean) => void
  disabled?: boolean
  className?: string
}

export function PolicyPicker({
  value,
  onChange,
  defaultSuffix,
  recursive,
  onRecursiveChange,
  disabled,
  className,
}: Props) {
  // 다른 방식으로 바꿨다 돌아와도 입력한 접미사·폴더가 남도록 따로 기억한다
  const [suffix, setSuffix] = useState(
    value.type === "KeepOriginal" ? value.suffix : defaultSuffix,
  )
  const [dir, setDir] = useState(value.type === "OutputFolder" ? value.dir : "")

  const select = (type: string) => {
    if (type === "KeepOriginal") onChange({ type, suffix })
    else if (type === "OutputFolder") onChange({ type, dir })
    else onChange({ type: "InPlaceWithBackup" })
  }

  const pickDir = async () => {
    const picked = await open({ directory: true }).catch(() => null)
    if (typeof picked === "string") {
      setDir(picked)
      onChange({ type: "OutputFolder", dir: picked })
    }
  }

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      <RadioGroup
        value={value.type}
        onValueChange={(v) => select(String(v))}
        disabled={disabled}
      >
        <label className="flex items-center gap-2">
          <RadioGroupItem value="InPlaceWithBackup" />
          원본 자리에 저장 (원본은 original 폴더로)
        </label>
        <label className="flex items-center gap-2">
          <RadioGroupItem value="KeepOriginal" />
          원본 옆에 저장
        </label>
        {value.type === "KeepOriginal" && (
          <Input
            aria-label="접미사"
            className="ml-6 w-40"
            value={suffix}
            onChange={(e) => {
              setSuffix(e.target.value)
              onChange({ type: "KeepOriginal", suffix: e.target.value })
            }}
            disabled={disabled}
          />
        )}
        <label className="flex items-center gap-2">
          <RadioGroupItem value="OutputFolder" />
          다른 폴더에 저장
        </label>
        {value.type === "OutputFolder" && (
          <div className="ml-6 flex items-center gap-2">
            <span
              className="text-muted-foreground truncate text-xs"
              title={dir}
            >
              {dir || "폴더를 선택하세요"}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={pickDir}
              disabled={disabled}
            >
              선택
            </Button>
          </div>
        )}
      </RadioGroup>
      <label className="flex items-center gap-2">
        <Checkbox
          checked={recursive}
          onCheckedChange={(c) => onRecursiveChange(c === true)}
          disabled={disabled}
        />
        하위 폴더 포함
      </label>
    </div>
  )
}
