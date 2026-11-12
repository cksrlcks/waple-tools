import { useState } from "react"
import { IconChevronDown } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { QUICK_SNIPPETS } from "../config/regex-snippets"
import { RegexCheatsheetDialog } from "./regex-cheatsheet-dialog"
import { SnippetButton } from "./snippet-button"

type Props = {
  onInsert: (insert: string) => void
  className?: string
}

/** 정규식 치트 시트: 자주 쓰는 토큰 한 줄 + [치트 시트]로 분류별 전체(모달) */
export function RegexCheatsheet({ onInsert, className }: Props) {
  const [open, setOpen] = useState(false)
  return (
    <div className={cn("flex flex-wrap items-center gap-1.5", className)}>
      <Button
        variant="outline"
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
        className="mr-1"
      >
        치트 시트
        <IconChevronDown data-icon="inline-end" stroke={1.75} />
      </Button>
      {QUICK_SNIPPETS.map((s) => (
        <SnippetButton key={s.label} snippet={s} onInsert={onInsert} />
      ))}
      <RegexCheatsheetDialog
        open={open}
        onOpenChange={setOpen}
        onInsert={onInsert}
      />
    </div>
  )
}
