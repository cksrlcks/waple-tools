import { IconCopy, IconDownload } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { copyImage } from "../model/copy-image"
import { saveWithDialog } from "../model/save-with-dialog"

type Props = {
  /** `data:image/png;base64,…` */
  src: string
  /** 저장할 때 기본 파일 이름. 비우면 [이미지 복사]·[저장]을 숨긴다 */
  name: string
  className?: string
}

export function ImageBlock({ src, name, className }: Props) {
  const comma = src.indexOf(",")
  const mime = src.slice(5, src.indexOf(";")) || "image/png"
  const base64 = src.slice(comma + 1)

  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex shrink-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      {/* 이름이 없으면 보기만 하는 그림: 머리줄(형식·복사·저장) 없이 */}
      {name && (
        <header className="flex h-9 shrink-0 items-center justify-between pr-1.5 pl-3">
          <span className="text-stage-muted text-2xs uppercase">
            {mime.replace("image/", "")}
          </span>
          <div className="flex gap-1">
            <Button size="sm" onClick={() => copyImage(src)}>
              <IconCopy data-icon="inline-start" stroke={1.75} />
              이미지 복사
            </Button>
            <Button
              size="sm"
              onClick={() => saveWithDialog(name, base64, true)}
            >
              <IconDownload data-icon="inline-start" stroke={1.75} />
              저장
            </Button>
          </div>
        </header>
      )}
      <div className={cn("flex justify-center px-3 pb-3", !name && "pt-3")}>
        {/* 칸 경계가 흐려지지 않게 줄여 보여도 픽셀 그대로 */}
        <img
          src={src}
          alt=""
          className="max-h-72 max-w-full object-contain [image-rendering:pixelated]"
        />
      </div>
    </section>
  )
}
