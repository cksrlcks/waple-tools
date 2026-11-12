import type { Span } from "@/entities/tool"

/** 화면에는 앞부분만 그린다 (수 MB 문자열은 그리기가 느리다). 복사는 전체를 한다 */
export function clipText(text: string, max: number): string {
  return text.length <= max ? text : `${text.slice(0, max)}…`
}

/** 숫자 블록 값: 천 단위 구분 + 단위 */
export function formatNumber(value: number, unit: string | null): string {
  const n = value.toLocaleString("ko-KR")
  return unit ? `${n} ${unit}` : n
}

/** 강조 조각도 이은 글자 수 `max`까지만 그린다. 잘리면 끝에 … */
export function clipSpans(spans: Span[], max: number): Span[] {
  const out: Span[] = []
  let left = max
  for (const s of spans) {
    if (s.text.length > left) {
      out.push({ ...s, text: clipText(s.text, left) })
      break
    }
    out.push(s)
    left -= s.text.length
  }
  return out
}
