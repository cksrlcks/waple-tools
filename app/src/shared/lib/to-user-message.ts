/** Rust 명령은 한국어 문구로 실패한다. 한국어가 아닌 오류(Tauri 내부 등)는 기본 문구로 바꾼다. */
export function toUserMessage(error: unknown): string {
  return typeof error === "string" && /[가-힣]/.test(error)
    ? error
    : "작업을 실행할 수 없습니다."
}
