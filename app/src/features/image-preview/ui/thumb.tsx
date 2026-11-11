import {
  IconFile,
  IconFileTypePdf,
  IconFolder,
  IconPhoto,
} from "@tabler/icons-react"

import type { PathStat } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { useThumbnail } from "../api/use-thumbnail"

/** 썸네일을 만들 수 있는 형식 (그 밖은 아이콘) */
const IMAGE = /\.(png|jpe?g|gif|webp|bmp|tiff?|ico|svg)$/i

type Props = {
  item: PathStat
  className?: string
}

/** 작업대 필름 띠의 썸네일 (폴더·PDF·읽을 수 없는 파일은 아이콘). 크기는 className으로 */
export function Thumb({ item, className }: Props) {
  const image = IMAGE.test(item.path)
  const thumb = useThumbnail(item.path, item.exists && !item.isDir && image)
  const box = cn(
    "bg-stage-foreground/8 text-stage-muted flex size-full items-center justify-center overflow-hidden",
    className,
  )
  if (item.isDir || !image) {
    const Icon = item.isDir
      ? IconFolder
      : /\.pdf$/i.test(item.path)
        ? IconFileTypePdf
        : IconFile
    return (
      <span className={box}>
        <Icon className="size-6" stroke={1.5} />
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
