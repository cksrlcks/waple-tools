import { IconFolder, IconPhoto } from "@tabler/icons-react"

import type { PathStat } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { useThumbnail } from "../api/use-thumbnail"

type Props = {
  item: PathStat
  className?: string
}

/** 작업대 필름 띠의 썸네일 (폴더·읽을 수 없는 파일은 아이콘). 크기는 className으로 */
export function Thumb({ item, className }: Props) {
  const thumb = useThumbnail(item.path, item.exists && !item.isDir)
  const box = cn(
    "bg-stage-foreground/8 text-stage-muted flex size-full items-center justify-center overflow-hidden",
    className,
  )
  if (item.isDir) {
    return (
      <span className={box}>
        <IconFolder className="size-6" stroke={1.5} />
      </span>
    )
  }
  if (!thumb.data) {
    return (
      <span className={box}>
        <IconPhoto className="size-5" stroke={1.5} />
      </span>
    )
  }
  return <img src={thumb.data} alt="" className={cn(box, "object-cover")} />
}
