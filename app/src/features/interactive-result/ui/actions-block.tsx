import { IconCopy, IconDownload } from "@tabler/icons-react"

import type { ActionItem } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { copyImage } from "../model/copy-image"
import { copyText } from "../model/copy-text"
import { saveWithDialog } from "../model/save-with-dialog"

type Props = {
  items: ActionItem[]
  className?: string
}

const run = (a: ActionItem) => {
  if (a.kind === "copyText") copyText(a.text)
  else if (a.kind === "copyImage") copyImage(a.src)
  else saveWithDialog(a.name, a.text, false)
}

/** 버튼 묶음: 글 복사 · 그림 복사 · 글 저장 (넘치면 줄바꿈) */
export function ActionsBlock({ items, className }: Props) {
  return (
    <div className={cn("flex shrink-0 flex-wrap gap-2", className)}>
      {items.map((a, i) => {
        const Icon = a.kind === "saveText" ? IconDownload : IconCopy
        return (
          <Button key={i} variant="outline" size="sm" onClick={() => run(a)}>
            <Icon data-icon="inline-start" stroke={1.75} />
            {a.label}
          </Button>
        )
      })}
    </div>
  )
}
