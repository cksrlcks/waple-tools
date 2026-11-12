/** 선택 영역(`start`~`end`)을 `insert`로 바꾼 글과 그 뒤 커서 위치. 괄호 쌍(`()`, `[]`, `(?:)`)이면 커서를 괄호 안에 둔다 */
export function insertAt(
  value: string,
  start: number,
  end: number,
  insert: string,
): { value: string; caret: number } {
  const next = value.slice(0, start) + insert + value.slice(end)
  // 빈 괄호로 끝나면 (`()`, `[]`, `[^]`, `(?:)`, `(?<name>)` …) 커서를 괄호 안에
  const inside = /(\(|\(\?(:|=|!|<=|<!|<name>)|\[\^?)[)\]]$/.test(insert)
  return { value: next, caret: start + insert.length - (inside ? 1 : 0) }
}
