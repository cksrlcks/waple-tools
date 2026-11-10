import {
  CATEGORY_LABELS,
  CATEGORY_ORDER,
  type ToolEntry,
} from "@/entities/tool"

export type ToolGroup = { key: string; label: string; tools: ToolEntry[] }

/** 즐겨찾기 그룹(있으면) + 고정 순서 카테고리 그룹. 빈 그룹은 뺀다. */
export function groupTools(
  tools: ToolEntry[],
  favorites: string[],
): ToolGroup[] {
  const byId = new Map(tools.map((t) => [t.id, t]))
  const fav = favorites.flatMap((id) => byId.get(id) ?? [])
  const groups: ToolGroup[] = fav.length
    ? [{ key: "favorites", label: "즐겨찾기", tools: fav }]
    : []
  for (const c of CATEGORY_ORDER) {
    const inCat = tools.filter((t) => t.category === c)
    if (inCat.length)
      groups.push({ key: c, label: CATEGORY_LABELS[c], tools: inCat })
  }
  return groups
}
