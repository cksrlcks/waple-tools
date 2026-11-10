/** Rust 명령은 한국어 문구(string)로 실패한다. 그 밖의 오류는 기본 문구로 바꾼다. */
export function toUserMessage(error: unknown): string {
  return typeof error === "string" && error
    ? error
    : "작업을 실행할 수 없습니다."
}
