/**
 * 한 화면에 인라인 SVG가 여러 개면 body 안 `id`가 서로 부딪힌다.
 * 화면에 그릴 때만 id와 그 참조(`url(#x)`, `href="#x"`)에 접두어를 붙인다 (복사·저장 결과에는 쓰지 않음).
 */
export function scopeIds(svg: string, scope: string): string {
  return svg
    .replace(/(\s)id="([^"]+)"/g, `$1id="${scope}-$2"`)
    .replace(/url\(#([^)]+)\)/g, `url(#${scope}-$1)`)
    .replace(/href="#([^"]+)"/g, `href="#${scope}-$1"`)
}
