import type { DiffLine } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import { LINE_BG, MARK_BG, NUM_CELL, TEXT_CELL } from "../config/diff-styles"
import { SpanText } from "./span-text"

type Props = {
  line?: DiffLine
  num?: number
  /** 오른쪽 칸: 왼쪽과 사이에 세로 선 */
  divider?: boolean
}

/** 좌우 비교의 한쪽 (번호 칸 + 글 칸). 짝이 없으면 빈 칸 */
export function DiffSide({ line, num, divider }: Props) {
  const bg = line ? LINE_BG[line.tag] : "bg-stage-foreground/4"
  return (
    <>
      <td
        className={cn(
          NUM_CELL,
          "w-12",
          bg,
          divider && "border-stage-foreground/15 border-l",
        )}
      >
        {num}
      </td>
      <td className={cn(TEXT_CELL, bg)}>
        {line && (
          <SpanText spans={line.spans} markClass={() => MARK_BG[line.tag]} />
        )}
      </td>
    </>
  )
}
