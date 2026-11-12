import type { DiffLine } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

import {
  LINE_BG,
  MARK_BG,
  NUM_CELL,
  SIGN,
  TEXT_CELL,
} from "../config/diff-styles"
import { toSplitRows } from "../lib/split-rows"
import { DiffSide } from "./diff-side"
import { SpanText } from "./span-text"

/** 화면에 그리는 최대 행 수 */
const SHOWN = 2_000

type Props = {
  layout: "split" | "unified"
  lines: DiffLine[]
  className?: string
}

/** 줄 번호가 있는 비교 결과. 지운 줄은 빨강, 넣은 줄은 초록, 줄 안에서 바뀐 글자는 더 진하게 */
export function DiffBlock({ layout, lines, className }: Props) {
  const split = layout === "split"
  const rows = split ? toSplitRows(lines) : []
  const total = split ? rows.length : lines.length
  return (
    <section
      className={cn(
        "bg-stage-foreground/8 flex min-h-0 flex-col overflow-hidden rounded-lg",
        className,
      )}
    >
      <div className="min-h-0 flex-1 overflow-auto py-2">
        <table
          className={cn(
            "w-full border-collapse font-mono text-xs leading-5",
            split && "table-fixed",
          )}
        >
          <tbody>
            {split
              ? rows.slice(0, SHOWN).map((r, i) => (
                  <tr key={i}>
                    <DiffSide line={r.left} num={r.left?.old} />
                    <DiffSide line={r.right} num={r.right?.new} divider />
                  </tr>
                ))
              : lines.slice(0, SHOWN).map((l, i) => (
                  <tr key={i} className={LINE_BG[l.tag]}>
                    <td className={NUM_CELL}>{l.old}</td>
                    <td className={NUM_CELL}>{l.new}</td>
                    <td className="text-stage-muted w-px pr-1 align-top select-none">
                      {SIGN[l.tag]}
                    </td>
                    <td className={TEXT_CELL}>
                      <SpanText
                        spans={l.spans}
                        markClass={() => MARK_BG[l.tag]}
                      />
                    </td>
                  </tr>
                ))}
          </tbody>
        </table>
      </div>
      {total > SHOWN && (
        <p className="text-stage-muted shrink-0 px-3 py-2 text-xs">
          화면에는 앞 {SHOWN.toLocaleString("ko-KR")}줄만 보입니다.
        </p>
      )}
    </section>
  )
}
