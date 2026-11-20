/**
 * 옵션 패널을 그릴지. 편집기 배치면 없고, `x-panel-when`(`when`)이 있으면
 * 그 옵션이 비어 있을 때 숨긴다.
 */
export function showPanel(
  when: string | null,
  options: Record<string, unknown>,
  hasLayout: boolean,
): boolean {
  return !hasLayout && (when === null || !!options[when])
}
