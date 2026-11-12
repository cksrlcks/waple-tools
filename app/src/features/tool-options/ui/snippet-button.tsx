import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import type { RegexSnippet } from "../config/regex-snippets"

type Props = {
  snippet: RegexSnippet
  onInsert: (insert: string) => void
  className?: string
}

/** 치트 시트의 토큰 하나: 누르면 패턴에 넣는다 */
export function SnippetButton({ snippet, onInsert, className }: Props) {
  return (
    <Button
      variant="outline"
      title={`${snippet.hint} — ${snippet.insert}`}
      // 입력칸의 커서·선택을 잃지 않게 눌러도 포커스를 옮기지 않는다
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => onInsert(snippet.insert)}
      className={cn("px-2.5 font-mono leading-none font-medium", className)}
    >
      {snippet.label}
    </Button>
  )
}
