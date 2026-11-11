import { IconCopy, IconDownload } from "@tabler/icons-react"
import { toast } from "sonner"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { clipText } from "../lib/clip-text"
import { saveWithDialog } from "../model/save-with-dialog"

/** 화면에 그리는 최대 글자 수 */
const SHOWN = 20_000

type Props = {
  code: string
  lang: string
  /** 있으면 [저장] 버튼, 기본 파일 이름 */
  name?: string
  className?: string
}

export function CodeBlock({ code, lang, name, className }: Props) {
  const copy = () =>
    navigator.clipboard
      .writeText(code)
      .then(() => toast.success("복사했습니다."))
      .catch(() => toast.error("복사할 수 없습니다."))
  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex min-h-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      <header className="flex h-9 shrink-0 items-center justify-between pr-1.5 pl-3">
        <span className="text-stage-muted text-2xs uppercase">{lang}</span>
        <div className="flex gap-1">
          <Button size="sm" onClick={copy}>
            <IconCopy data-icon="inline-start" stroke={1.75} />
            복사
          </Button>
          {name && (
            <Button size="sm" onClick={() => saveWithDialog(name, code, false)}>
              <IconDownload data-icon="inline-start" stroke={1.75} />
              저장
            </Button>
          )}
        </div>
      </header>
      <pre className="min-h-0 flex-1 overflow-auto px-3 pb-3 font-mono text-xs break-all whitespace-pre-wrap select-text">
        {clipText(code, SHOWN)}
      </pre>
      {code.length > SHOWN && (
        <p className="text-stage-muted shrink-0 px-3 pb-2 text-xs">
          화면에는 앞부분만 보입니다. 복사하면 전체가 복사됩니다.
        </p>
      )}
    </section>
  )
}
