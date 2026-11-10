import type { FileResult } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  result: FileResult
  className?: string
}

export function FileResultRow({ result, className }: Props) {
  switch (result.status) {
    case "done":
      return <li className={className}>{result.outputs.join(", ")}</li>
    case "skipped":
      return (
        <li className={cn("text-muted-foreground", className)}>
          {result.input}: 건너뜀({result.reason})
        </li>
      )
    case "failed":
      return (
        <li className={cn("text-destructive", className)}>
          {result.input}: {result.message}
        </li>
      )
    case "cancelled":
      return (
        <li className={cn("text-muted-foreground", className)}>
          {result.input}: 취소됨
        </li>
      )
  }
}
