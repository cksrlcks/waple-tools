import { styleOf, type IconItem } from "@/entities/icon-set"

type Opts = {
  query: string
  style: string | null
  styles: Record<string, string>
}

/** 스타일(null이면 전체)과 검색어(공백으로 나눈 단어가 모두 이름에, 대소문자 무시)로 거른다 */
export function filterIcons(
  icons: IconItem[],
  { query, style, styles }: Opts,
): IconItem[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  return icons.filter(
    (i) =>
      (style === null || styleOf(i.name, styles) === style) &&
      words.every((w) => i.name.toLowerCase().includes(w)),
  )
}
