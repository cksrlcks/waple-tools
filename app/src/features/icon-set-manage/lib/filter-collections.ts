import type { Collection } from "@/entities/icon-set"

/** 이름·prefix에 검색어가 들어 있는 세트만 (대소문자 무시) */
export function filterCollections(
  list: Collection[],
  query: string,
): Collection[] {
  const q = query.trim().toLowerCase()
  if (!q) return list
  return list.filter(
    (c) =>
      c.name.toLowerCase().includes(q) || c.prefix.toLowerCase().includes(q),
  )
}
