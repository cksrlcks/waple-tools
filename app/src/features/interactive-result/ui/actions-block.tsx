import { IconCopy, IconDownload } from "@tabler/icons-react"

import type { ActionItem } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { copyImage } from "../model/copy-image"
import { copyText } from "../model/copy-text"
import { saveWithDialog } from "../model/save-with-dialog"

/** `row`: 한 줄 오른쪽 정렬(기본), `grid`: 두 칸 격자에 칸 너비 버튼, 강조 버튼 먼저 (좁은 패널) */
export type ActionsLayout = "row" | "grid"

type Props = {
  items: ActionItem[]
  layout?: ActionsLayout
  className?: string
}

const run = (a: ActionItem) => {
  if (a.kind === "copyText") copyText(a.text)
  else if (a.kind === "copyImage") copyImage(a.src)
  else saveWithDialog(a.name, a.text, false)
}

/** 버튼 묶음: 글 복사 · 그림 복사 · 글 저장. 한 줄로, 좁으면 줄바꿈 대신 가로 스크롤 (p-1: 포커스 링이 안 잘리게) */
export function ActionsBlock({ items, layout = "row", className }: Props) {
  const grid = layout === "grid"
  // 격자는 강조 버튼을 앞으로 (sort는 안정 정렬: 나머지 순서는 그대로)
  const shown = grid
    ? [...items].sort((a, b) => Number(!!b.primary) - Number(!!a.primary))
    : items
  return (
    <div
      className={cn(
        "-m-1 flex shrink-0 p-1",
        !grid && "overflow-x-auto",
        className,
      )}
    >
      {/* row — ml-auto + w-max: 오른쪽 정렬, 넘치면 왼쪽부터 스크롤 */}
      <div
        className={cn(
          "gap-1.5",
          grid ? "grid w-full grid-cols-2" : "ml-auto flex w-max",
        )}
      >
        {shown.map((a, i) => {
          const Icon = a.kind === "saveText" ? IconDownload : IconCopy
          return (
            <Button
              key={i}
              variant={a.primary ? "default" : "outline"}
              // 격자(좁은 패널)는 칸 너비라 납작해 보이지 않게 기본 높이(h-8)
              size={grid ? "default" : "sm"}
              // 양옆 패딩을 같게, 줄 높이를 아이콘에 맞춰 글자·아이콘 세로 가운데 (sm 기본은 leading-none·왼쪽 pl-1.5)
              className={cn(
                "shrink-0 gap-1.5 px-2.5 leading-4 has-data-[icon=inline-start]:pl-2.5",
                grid && "w-full min-w-0",
              )}
              onClick={() => run(a)}
            >
              <Icon
                data-icon="inline-start"
                stroke={1.75}
                className="size-4 shrink-0"
              />
              <span className={cn("leading-4", grid && "truncate")}>
                {a.label}
              </span>
            </Button>
          )
        })}
      </div>
    </div>
  )
}
