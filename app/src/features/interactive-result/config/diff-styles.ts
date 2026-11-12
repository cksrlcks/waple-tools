/** 비교 줄 바탕: 지운 줄 빨강, 넣은 줄 초록 */
export const LINE_BG = {
  equal: "",
  delete: "bg-destructive/15",
  insert: "bg-success/15",
} as const

/** 줄 안에서 바뀐 글자 (`mark`) */
export const MARK_BG = {
  equal: "",
  delete: "bg-destructive/45",
  insert: "bg-success/45",
} as const

export const SIGN = { equal: " ", delete: "−", insert: "+" } as const

/** 줄 번호 칸 */
export const NUM_CELL =
  "text-stage-muted w-px px-2 text-right align-top whitespace-nowrap tabular-nums select-none"

/** 글 칸 */
export const TEXT_CELL =
  "px-2 align-top break-all whitespace-pre-wrap select-text"
