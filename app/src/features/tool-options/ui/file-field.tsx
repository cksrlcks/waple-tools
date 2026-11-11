import { IconX } from "@tabler/icons-react"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"

import { baseName } from "@/shared/lib/base-name"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

type Props = {
  id: string
  /** 필드 이름 (지우기 버튼의 읽기 이름에 쓴다) */
  label: string
  /** 절대 경로, 빈 문자열 = 선택 안 함 */
  value: string
  /** 파일 선택 창에서 보일 확장자 (비면 모두) */
  extensions: string[]
  onChange: (value: string) => void
  disabled?: boolean
  className?: string
}

/** 파일 하나 고르기: 버튼에 고른 파일 이름, 옆에 지우기 */
export function FileField({
  id,
  label,
  value,
  extensions,
  onChange,
  disabled,
  className,
}: Props) {
  const pick = async () => {
    try {
      const filters = extensions.length
        ? [{ name: "지원 형식", extensions }]
        : undefined
      const picked = await open({ multiple: false, directory: false, filters })
      if (typeof picked === "string") onChange(picked)
    } catch {
      toast.error("파일을 선택할 수 없습니다.")
    }
  }

  return (
    <div className={cn("flex w-40 items-center gap-1", className)}>
      <Button
        id={id}
        variant="outline"
        className="min-w-0 flex-1 justify-start"
        title={value || undefined}
        aria-describedby={`${id}-name`}
        onClick={pick}
        disabled={disabled}
      >
        <span
          id={`${id}-name`}
          className={cn("truncate", !value && "text-muted-foreground")}
        >
          {value ? baseName(value) : "파일 선택…"}
        </span>
      </Button>
      {value && (
        <Button
          variant="ghost"
          size="icon"
          aria-label={`${label} 지우기`}
          onClick={() => onChange("")}
          disabled={disabled}
        >
          <IconX className="size-4" stroke={1.75} />
        </Button>
      )}
    </div>
  )
}
