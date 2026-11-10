import type { PathStat } from "@/entities/tool"

/**
 * 넣은 파일들의 확장자 (소문자, 중복 없음). 옵션 보일지 판단(`$inputExt`)에 쓴다.
 * 폴더는 안에 뭐가 있는지 모르므로 도구가 받는 확장자 전부로 본다.
 */
export function inputExts(items: PathStat[], toolExts: string[]): string[] {
  const out = new Set<string>()
  for (const i of items) {
    if (i.isDir) {
      for (const e of toolExts) out.add(e.toLowerCase())
      continue
    }
    const m = /\.([^.\/]+)$/.exec(i.path)
    if (m?.[1]) out.add(m[1].toLowerCase())
  }
  return [...out]
}
