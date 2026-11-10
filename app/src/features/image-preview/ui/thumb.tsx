import { IconFolder, IconPhoto } from "@tabler/icons-react"

import type { PathStat } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { useThumbnail } from "../api/use-thumbnail"

type Props = {
  item: PathStat
  className?: string
}

/** 입력 목록용 32px 썸네일 (폴더·읽을 수 없는 파일은 아이콘) */
export function Thumb({ item, className }: Props) {
  const thumb = useThumbnail(item.path, item.exists && !item.isDir)
  const box = cn(
    "flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-sm",
    className,
  )
  if (item.isDir) {
    return (
      <span className={box}>
        <IconFolder className="text-muted-foreground size-5" stroke={1.75} />
      </span>
    )
  }
  if (!thumb.data) {
    return (
      <span className={cn(box, "bg-muted")}>
        <IconPhoto className="text-muted-foreground size-4" stroke={1.75} />
      </span>
    )
  }
  return (
    <img
      src={thumb.data}
      alt=""
      className={cn(box, "shadow-item object-cover")}
    />
  )
}
