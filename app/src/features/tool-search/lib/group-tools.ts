import type { ToolEntry } from "@/entities/tool"

export type ToolGroup = { key: string; label: string; tools: ToolEntry[] }

/** 탭 안 목록: 즐겨찾기(그 탭의 것만, 즐겨찾기 순서) + 나머지 "도구". 빈 그룹은 뺀다. */
export function groupTools(
  tools: ToolEntry[],
  favorites: string[],
): ToolGroup[] {
  const byId = new Map(tools.map((t) => [t.id, t]))
  const fav = favorites.flatMap((id) => byId.get(id) ?? [])
  const rest = tools.filter((t) => !favorites.includes(t.id))
  const groups: ToolGroup[] = []
  if (fav.length)
    groups.push({ key: "favorites", label: "즐겨찾기", tools: fav })
  if (rest.length) groups.push({ key: "all", label: "도구", tools: rest })
  return groups
}
