import type { PathStat } from "@/entities/tool"

/**
 * 도구가 받는 형식만 남긴다. 실행 단계(core `supported`)와 같은 규칙:
 * 확장자 목록이 비면 모두 받고, 아니면 소문자 확장자가 목록에 있어야 한다.
 * 폴더는 안을 실행할 때 거르므로 그대로 받는다.
 */
export function filterSupported(
  stats: PathStat[],
  exts: string[],
): { ok: PathStat[]; rejected: number } {
  if (exts.length === 0) return { ok: stats, rejected: 0 }
  const ok = stats.filter((s) => {
    if (s.isDir) return true
    const m = /\.([^.\\/]+)$/.exec(s.path)
    return !!m?.[1] && exts.includes(m[1].toLowerCase())
  })
  return { ok, rejected: stats.length - ok.length }
}
