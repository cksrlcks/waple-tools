import type { ResourceItem } from "@/entities/tool"

/** 리소스 목록 탭: 받은 항목만 / 전체 */
export type ResourceTab = "installed" | "all"

/** 고른 탭의 항목만 (순서 그대로) */
export function filterByTab(
  items: ResourceItem[],
  installedIds: Pick<ReadonlySet<string>, "has">,
  tab: ResourceTab,
): ResourceItem[] {
  return tab === "all" ? items : items.filter((i) => installedIds.has(i.id))
}
