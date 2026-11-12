import type { RenamePlan } from "@/entities/tool"

export type RenameSummary = {
  changed: number
  unchanged: number
  conflict: number
  /** 실행할 수 없는 이유 (없으면 실행 가능) */
  blockedReason: string | null
}

/** 표 상태 개수와 실행 가능 여부. `current`가 아니면(옵션을 바꾼 직후) 새 표를 기다린다 */
export function renameSummary(
  plan: RenamePlan | undefined,
  current: boolean,
): RenameSummary {
  const s: RenameSummary = {
    changed: 0,
    unchanged: 0,
    conflict: 0,
    blockedReason: null,
  }
  for (const r of plan?.rows ?? []) s[r.status]++
  s.blockedReason =
    !plan || !current
      ? "미리보기를 만드는 중입니다."
      : s.conflict > 0
        ? "충돌하는 이름이 있어 바꿀 수 없습니다."
        : s.changed === 0
          ? "바뀌는 이름이 없습니다."
          : null
  return s
}
