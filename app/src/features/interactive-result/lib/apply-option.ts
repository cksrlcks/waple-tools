/** 옵션 하나를 바꾸고, `resets`의 옵션은 기본값으로 되돌린다 (기본값이 없으면 지운다) */
export function applyOption(
  opts: Record<string, unknown>,
  key: string,
  value: unknown,
  resets: string[] = [],
  defaults: Record<string, unknown> = {},
): Record<string, unknown> {
  const next = { ...opts, [key]: value }
  for (const k of resets) {
    if (k in defaults) next[k] = defaults[k]
    else delete next[k]
  }
  return next
}
