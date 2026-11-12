import { IconAlertCircleFilled, IconX } from "@tabler/icons-react"

import type { RenameRow } from "@/entities/tool"
import { baseName } from "@/shared/lib/base-name"
import { cn } from "@/shared/lib/utils"

type Props = {
  row: RenameRow
  /** 넣은 파일 그대로인 줄만 뺄 수 있다 (폴더에서 펼친 파일은 폴더째 뺀다) */
  onRemove?: () => void
  disabled?: boolean
  className?: string
}

/** 미리보기 표 한 줄: 현재 이름 · 새 이름 · 상태 (그대로는 흐리게, 충돌은 이유와 함께 빨갛게) */
export function RenameTableRow({ row, onRemove, disabled, className }: Props) {
  const conflict = row.status === "conflict"
  const unchanged = row.status === "unchanged"
  return (
    <tr
      className={cn(
        "group h-8.5",
        conflict ? "bg-destructive/10" : "hover:bg-stage-foreground/5",
        className,
      )}
    >
      <td className="truncate px-3" title={row.from}>
        {baseName(row.from)}
      </td>
      <td
        className={cn(
          "truncate px-3",
          unchanged && "text-stage-muted",
          conflict && "text-destructive font-medium",
          row.status === "changed" && "font-medium",
        )}
        title={row.to}
      >
        {row.name || "(빈 이름)"}
      </td>
      <td
        className={cn(
          "truncate px-3 text-xs",
          conflict
            ? "text-destructive"
            : row.status === "changed"
              ? "text-success"
              : "text-stage-muted",
        )}
        title={conflict ? row.reason : undefined}
      >
        {conflict ? (
          <span className="flex items-center gap-1">
            <IconAlertCircleFilled className="size-3.5 shrink-0" />
            <span className="truncate">{row.reason}</span>
          </span>
        ) : unchanged ? (
          "그대로"
        ) : (
          "바뀜"
        )}
      </td>
      <td className="px-1">
        {onRemove && (
          <button
            type="button"
            aria-label="목록에서 빼기"
            onClick={onRemove}
            disabled={disabled}
            className="text-stage-muted hover:text-stage-foreground hover:bg-stage-foreground/10 invisible flex size-6 items-center justify-center rounded group-hover:visible focus-visible:visible disabled:hidden"
          >
            <IconX className="size-3.5" stroke={2} />
          </button>
        )}
      </td>
    </tr>
  )
}
