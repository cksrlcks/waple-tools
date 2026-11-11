import { IconCopy, IconDownload } from "@tabler/icons-react"
import { toast } from "sonner"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { saveWithDialog } from "../model/save-with-dialog"

type Props = {
  /** `data:image/png;base64,…` */
  src: string
  /** 저장할 때 기본 파일 이름 */
  name: string
  className?: string
}

export function ImageBlock({ src, name, className }: Props) {
  const comma = src.indexOf(",")
  const mime = src.slice(5, src.indexOf(";")) || "image/png"
  const base64 = src.slice(comma + 1)

  const copy = () => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0))
    navigator.clipboard
      .write([new ClipboardItem({ [mime]: new Blob([bytes], { type: mime }) })])
      .then(() => toast.success("이미지를 복사했습니다."))
      .catch(() => toast.error("복사할 수 없습니다."))
  }

  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex shrink-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      <header className="flex h-9 shrink-0 items-center justify-between pr-1.5 pl-3">
        <span className="text-stage-muted text-2xs uppercase">
          {mime.replace("image/", "")}
        </span>
        <div className="flex gap-1">
          <Button size="sm" onClick={copy}>
            <IconCopy data-icon="inline-start" stroke={1.75} />
            이미지 복사
          </Button>
          <Button size="sm" onClick={() => saveWithDialog(name, base64, true)}>
            <IconDownload data-icon="inline-start" stroke={1.75} />
            저장
          </Button>
        </div>
      </header>
      <div className="flex justify-center px-3 pb-3">
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
