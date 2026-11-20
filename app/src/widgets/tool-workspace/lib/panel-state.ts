import type { Block } from "@/entities/tool"
import type { Field } from "@/features/tool-options"

import { hiddenOptions, optionHints } from "./option-hints"

/** 메인 → 패널 창 `panel:state`: 패널 창이 그리는 것 전부 (JSON으로 보낸다) */
export type PanelState = {
  title: string
  fields: Field[]
  options: Record<string, unknown>
  /** 패널 블록 (`x-panel-blocks`) */
  blocks: Block[]
  /** 값이 null인 칸에 보여 줄 값 (`optionHints`) */
  hints: Record<string, unknown>
  /** 지금 결과에서 바꿀 수 없어 감춘 옵션 키 (`fields`에서 이미 뺐다) */
  hidden: string[]
  /** 결과를 다시 만드는 중: 미리보기·버튼을 흐리게 */
  fading: boolean
  error: string | null
}

/** 패널 창에 보낼 상태: 결과(`data`)의 `optionHints`로 힌트·숨김을 모으고 숨길 칸을 뺀다 */
export function panelState({
  title,
  fields,
  options,
  blocks,
  data,
  fetching,
  error,
}: {
  title: string
  fields: Field[]
  options: Record<string, unknown>
  blocks: Block[]
  data: Block[] | undefined
  fetching: boolean
  error: string | null
}): PanelState {
  const hidden = hiddenOptions(data)
  return {
    title,
    fields: fields.filter((f) => !hidden.includes(f.key)),
    options,
    blocks,
    hints: optionHints(data),
    hidden,
    fading: fetching,
    error,
  }
}

/** 옵션 폼이 바꾼 칸: `next`에서 `prev`와 값이 다른 [키, 값] (패널 창 → 메인 `panel:set`) */
export function changedEntries(
  prev: Record<string, unknown>,
  next: Record<string, unknown>,
): [string, unknown][] {
  return Object.entries(next).filter(([k, v]) => !Object.is(prev[k], v))
}
