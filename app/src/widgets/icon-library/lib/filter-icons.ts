import { styleOf, type IconItem } from "@/entities/icon-set"

type Opts = {
  query: string
  style: string | null
  styles: Record<string, string>
}

/**
 * 스타일(null이면 전체)과 검색어(공백으로 나눈 단어가 모두 이름에, 대소문자 무시)로 거른다.
 * styles에 "" 키가 없으면(ri 등) 접미어 없는 아이콘은 어느 스타일에서나 보인다.
 */
export function filterIcons(
  icons: IconItem[],
  { query, style, styles }: Opts,
): IconItem[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  const plainEverywhere = !("" in styles)
  const styleOk = (name: string) => {
    if (style === null) return true
    const s = styleOf(name, styles)
    return s === style || (plainEverywhere && s === "")
  }
  return icons.filter(
    (i) =>
      styleOk(i.name) && words.every((w) => i.name.toLowerCase().includes(w)),
  )
}
