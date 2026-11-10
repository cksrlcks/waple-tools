/** 앱 업데이트 버튼의 상태 */
export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "failed" }
  | { kind: "available"; version: string }
  | { kind: "installing"; percent: number | null }

/** 버튼 글자 */
export function updateLabel(s: UpdateStatus): string {
  switch (s.kind) {
    case "idle":
      return "업데이트 확인"
    case "checking":
      return "확인 중…"
    case "latest":
      return "최신입니다."
    case "failed":
      return "확인할 수 없습니다."
    case "available":
      return `v${s.version} 설치`
    case "installing":
      return s.percent === null ? "설치 중…" : `설치 중… ${s.percent}%`
  }
}

/** 받은 바이트로 진행률(%). 전체 크기를 모르면 null */
export function percentOf(
  done: number,
  total: number | undefined,
): number | null {
  if (!total) return null
  return Math.min(100, Math.round((done / total) * 100))
}
