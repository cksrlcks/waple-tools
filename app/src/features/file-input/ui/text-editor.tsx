import { useId, useRef } from "react"
import { IconPlus } from "@tabler/icons-react"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"

import type { Span } from "@/entities/tool"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { readTextFile } from "../api/read-text-file"
import { useFileDrop } from "../model/use-file-drop"

type Props = {
  label: string
  /** 비어 있을 때 보이는 안내 */
  placeholder?: string
  value: string
  onChange: (value: string) => void
  /** 글 안에서 강조할 구간 (조각을 이은 글이 `value`와 같을 때만 그린다) */
  marks?: Span[]
  className?: string
}

/** 편집기 글자 배치. 강조 층과 똑같아야 구간이 글자에 겹친다 */
const TEXT_LAYOUT =
  "[scrollbar-gutter:stable] px-3 pb-3 font-mono text-xs leading-5 break-words whitespace-pre-wrap"

/** 작업대의 큰 글 편집기. [파일 열기]로 고르거나 이 칸 위에 끌어 놓으면 파일 내용(UTF-8)을 넣는다 */
export function TextEditor({
  label,
  placeholder,
  value,
  onChange,
  marks,
  className,
}: Props) {
  const id = useId()
  const ref = useRef<HTMLElement>(null)
  const backdrop = useRef<HTMLDivElement>(null)
  const shownMarks =
    marks && marks.map((m) => m.text).join("") === value ? marks : null

  const load = (path: string) =>
    readTextFile(path)
      .then(onChange)
      .catch((err) => toast.error(toUserMessage(err)))

  const { over } = useFileDrop(
    (paths) => {
      if (paths.length > 1) toast.info("한 번에 한 파일만 엽니다.")
      if (paths[0]) load(paths[0])
    },
    true,
    ref,
  )

  const pick = async () => {
    try {
      const picked = await open({ multiple: false, directory: false })
      if (typeof picked === "string") load(picked)
    } catch {
      toast.error("파일을 선택할 수 없습니다.")
    }
  }

  return (
    <section
      ref={ref}
      className={cn(
        "bg-card text-card-foreground focus-within:ring-primary/40 flex min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border transition-colors focus-within:ring-2",
        over && "bg-primary/15 ring-primary ring-2",
        className,
      )}
    >
      <header className="flex h-9 shrink-0 items-center px-3">
        <label htmlFor={id} className="truncate text-xs font-medium">
          {over ? "놓으면 파일 내용을 넣습니다" : label}
        </label>
      </header>
      <div className="relative min-h-0 flex-1">
        {shownMarks && (
          <div
            ref={backdrop}
            aria-hidden
            className={cn(
              "absolute inset-0 overflow-hidden text-transparent",
              TEXT_LAYOUT,
            )}
          >
            {shownMarks.map((m, i) =>
              m.mark === undefined ? (
                <span key={i}>{m.text}</span>
              ) : (
                <mark
                  key={i}
                  className={cn(
                    "rounded-sm text-transparent",
                    m.mark % 2 === 0 ? "bg-primary/25" : "bg-primary/45",
                  )}
                >
                  {m.text}
                </mark>
              ),
            )}
            {/* 끝 줄바꿈도 높이를 갖게 */}
            {"\n "}
          </div>
        )}
        <textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onScroll={(e) => {
            if (backdrop.current)
              backdrop.current.scrollTop = e.currentTarget.scrollTop
          }}
          placeholder={placeholder ?? "여기에 입력하거나 파일을 끌어 놓으세요"}
          spellCheck={false}
          className={cn(
            "placeholder:text-muted-foreground relative size-full resize-none bg-transparent outline-none",
            TEXT_LAYOUT,
          )}
        />
      </div>
      <footer className="bg-muted flex h-10 shrink-0 items-center border-t px-1.5">
        <Button
          size="sm"
          variant="ghost"
          onClick={pick}
          className="text-xs leading-none"
        >
          <IconPlus data-icon="inline-start" stroke={1.75} className="size-3" />
          파일 열기
        </Button>
      </footer>
    </section>
  )
}
