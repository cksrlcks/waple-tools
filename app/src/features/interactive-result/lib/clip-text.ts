/** 화면에는 앞부분만 그린다 (수 MB 문자열은 그리기가 느리다). 복사는 전체를 한다 */
export function clipText(text: string, max: number): string {
  return text.length <= max ? text : `${text.slice(0, max)}…`
}

/** 숫자 블록 값: 천 단위 구분 + 단위 */
export function formatNumber(value: number, unit: string | null): string {
  const n = value.toLocaleString("ko-KR")
  return unit ? `${n} ${unit}` : n
}
