import type { OutputPolicy } from "@/entities/tool"

export type CanRun = { ok: true } | { ok: false; reason: string }

/** 파일 이름에 쓸 수 없는 문자(경로 구분자 포함)와 `..` — 결과가 다른 폴더나 대체 스트림으로 가는 것을 막는다 */
const BAD_SUFFIX = /[\\/:*?"<>|]|\.\./

/** @param busy 실행 중이거나 되돌리는 중 */
export function canRun(
  inputs: string[],
  policy: OutputPolicy,
  busy: boolean,
): CanRun {
  if (busy) return { ok: false, reason: "작업 중입니다." }
  if (inputs.length === 0)
    return { ok: false, reason: "처리할 파일을 추가하세요." }
  if (policy.type === "OutputFolder" && !policy.dir.trim())
    return { ok: false, reason: "저장할 폴더를 선택하세요." }
  if (policy.type === "KeepOriginal") {
    if (!policy.suffix.trim())
      return { ok: false, reason: "접미사를 입력하세요." }
    if (BAD_SUFFIX.test(policy.suffix))
      return { ok: false, reason: "접미사에 쓸 수 없는 문자가 있습니다." }
  }
  return { ok: true }
}
