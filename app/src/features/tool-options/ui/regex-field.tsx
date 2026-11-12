import { useRef } from "react"

import { cn } from "@/shared/lib/utils"

import { insertAt } from "../lib/insert-at"
import { RegexCheatsheet } from "./regex-cheatsheet"

type Props = {
  label: string
  value: string
  onChange: (value: string) => void
  /** 슬래시 뒤에 보여 줄 플래그 글자 (예: `gi`) */
  flags?: string
  className?: string
}

/** `format: "regex"` 입력칸: 치트 시트 + `/ 패턴 / 플래그` */
export function RegexField({
  label,
  value,
  onChange,
  flags = "",
  className,
}: Props) {
  const ref = useRef<HTMLInputElement>(null)

  const insert = (text: string) => {
    const el = ref.current
    const start = el?.selectionStart ?? value.length
    const end = el?.selectionEnd ?? value.length
    const next = insertAt(value, start, end, text)
    onChange(next.value)
    requestAnimationFrame(() => {
      el?.focus()
      el?.setSelectionRange(next.caret, next.caret)
    })
  }

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <RegexCheatsheet onInsert={insert} />
      <div className="bg-card focus-within:border-primary focus-within:ring-primary/20 flex h-10 items-center gap-2 rounded-lg border px-3 font-mono text-sm focus-within:ring-2">
        <span aria-hidden className="text-primary">
          /
        </span>
        <input
          ref={ref}
          aria-label={label}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="정규 표현식을 입력…"
          spellCheck={false}
          autoComplete="off"
          className="placeholder:text-muted-foreground min-w-0 flex-1 bg-transparent outline-none"
        />
        <span aria-hidden className="text-primary">
          /
        </span>
        <span className="text-muted-foreground min-w-6">{flags}</span>
      </div>
    </div>
  )
}
