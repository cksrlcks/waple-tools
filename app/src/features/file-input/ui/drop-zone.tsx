import { open } from "@tauri-apps/plugin-dialog"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { useFileDrop } from "../model/use-file-drop"

type Props = {
  onAdd: (paths: string[]) => void
  onError: (message: string) => void
  disabled?: boolean
  className?: string
}

export function DropZone({
  onAdd,
  onError,
  disabled = false,
  className,
}: Props) {
  const { over } = useFileDrop(onAdd, !disabled)

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

  return (
    <div
      className={cn(
        "flex items-center justify-between gap-3 rounded-md border border-dashed px-4 py-5 transition-colors",
        over && !disabled && "border-primary bg-primary/10",
        className,
      )}
    >
      <p className="text-muted-foreground">파일이나 폴더를 끌어 놓으세요</p>
      <div className="flex gap-2">
        <Button
          variant="outline"
          onClick={() => pick(false)}
          disabled={disabled}
        >
          파일 선택
        </Button>
        <Button
          variant="outline"
          onClick={() => pick(true)}
          disabled={disabled}
        >
          폴더 선택
        </Button>
      </div>
    </div>
  )
}
