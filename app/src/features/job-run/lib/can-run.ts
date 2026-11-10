import type { OutputPolicy } from "@/entities/tool"

export type CanRun = { ok: true } | { ok: false; reason: string }

export function canRun(
  inputs: string[],
  policy: OutputPolicy,
  running: boolean,
): CanRun {
  if (running) return { ok: false, reason: "실행 중입니다." }
  if (inputs.length === 0)
    return { ok: false, reason: "처리할 파일을 추가하세요." }
  if (policy.type === "OutputFolder" && !policy.dir.trim())
    return { ok: false, reason: "저장할 폴더를 선택하세요." }
  if (policy.type === "KeepOriginal" && !policy.suffix.trim())
    return { ok: false, reason: "접미사를 입력하세요." }
  return { ok: true }
}
