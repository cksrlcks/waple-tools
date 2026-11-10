import { IconFolderPlus, IconPhotoPlus, IconPlus } from "@tabler/icons-react"
import { open } from "@tauri-apps/plugin-dialog"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu"

import { useFileDrop } from "../model/use-file-drop"

type Props = {
  onAdd: (paths: string[]) => void
  onError: (message: string) => void
  /** hero: 빈 작업대 전체, tile: 필름 띠 끝의 작은 추가 칸 */
  variant?: "hero" | "tile"
  disabled?: boolean
  className?: string
}

/** 파일·폴더 넣기. 창 어디에 끌어 놓아도 받는다 (한 화면에 하나만 둔다) */
export function DropZone({
  onAdd,
  onError,
  variant = "hero",
  disabled = false,
  className,
}: Props) {
  const { over } = useFileDrop(onAdd, !disabled)
  const hot = over && !disabled

  const pick = async (directory: boolean) => {
    try {
      const picked = await open({ multiple: true, directory })
      if (picked) onAdd(picked)
    } catch {
      onError(
        directory ? "폴더를 선택할 수 없습니다." : "파일을 선택할 수 없습니다.",
      )
    }
  }

  if (variant === "tile") {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger
          disabled={disabled}
          aria-label="파일 추가"
          className={cn(
            "border-stage-foreground/20 text-stage-muted hover:border-stage-foreground/40 hover:text-stage-foreground focus-visible:ring-primary flex h-16 w-24 items-center justify-center rounded-lg border border-dashed transition-colors outline-none focus-visible:ring-2 disabled:opacity-50",
            hot && "border-primary bg-primary/15 text-stage-foreground",
            className,
          )}
        >
          <IconPlus className="size-5" stroke={1.75} />
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" className="min-w-36">
          <DropdownMenuItem onClick={() => pick(false)}>
            파일 추가…
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => pick(true)}>
            폴더 추가…
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    )
  }

  return (
    <div
      className={cn(
        "border-stage-foreground/15 m-4 flex flex-1 flex-col items-center justify-center gap-5 rounded-xl border border-dashed transition-colors",
        hot && "border-primary bg-primary/10",
        className,
      )}
    >
      <span
        className={cn(
          "bg-stage-foreground/8 text-stage-muted flex size-14 items-center justify-center rounded-2xl transition-colors",
          hot && "bg-primary text-primary-foreground",
        )}
      >
        <IconPhotoPlus className="size-7" stroke={1.5} />
      </span>
      <div className="flex flex-col items-center gap-1 text-center">
        <p className="text-base font-semibold">
          {hot ? "놓으면 추가됩니다" : "파일이나 폴더를 여기에 끌어 놓으세요"}
        </p>
        <p className="text-stage-muted text-sm">
          파일은 이 컴퓨터 안에서만 처리합니다
        </p>
      </div>
      <div className="flex gap-2">
        <Button size="lg" onClick={() => pick(false)} disabled={disabled}>
          <IconPhotoPlus stroke={1.75} />
          파일 선택
        </Button>
        <Button
          size="lg"
          variant="outline"
          onClick={() => pick(true)}
          disabled={disabled}
        >
          <IconFolderPlus stroke={1.75} />
          폴더 선택
        </Button>
      </div>
    </div>
  )
}
