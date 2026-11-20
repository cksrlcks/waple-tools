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

/** 버튼 묶음: 글 복사 · 그림 복사 · 글 저장. 한 줄로, 좁으면 줄바꿈 대신 가로 스크롤 (p-1: 포커스 링이 안 잘리게) */
export function ActionsBlock({ items, className }: Props) {
  return (
    <div
      className={cn("-m-1 flex shrink-0 overflow-x-auto p-1", className)}
    >
      {/* ml-auto + w-max: 오른쪽 정렬, 넘치면 왼쪽부터 스크롤 */}
      <div className="ml-auto flex w-max gap-1.5">
        {items.map((a, i) => {
          const Icon = a.kind === "saveText" ? IconDownload : IconCopy
          return (
            <Button
              key={i}
              variant="outline"
              size="sm"
              className="shrink-0"
              onClick={() => run(a)}
            >
              <Icon data-icon="inline-start" stroke={1.75} />
              {a.label}
            </Button>
          )
        })}
      </div>
    </div>
  )
}
