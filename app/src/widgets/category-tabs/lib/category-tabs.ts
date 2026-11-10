import { CATEGORY_LABELS, type ToolEntry } from "@/entities/tool"

export type CategoryTab = {
  key: string
  label: string
  disabled: boolean
  /** 탭 막대 오른쪽 끝에 붙인다 (설정) */
  end?: boolean
}

/** 주 탭(이미지·PDF·웹폰트·기타)은 항상 보이고 도구가 없으면 비활성. 개발용은 개발 빌드에서만 맨 끝 */
export function categoryTabs(
  tools: ToolEntry[],
  debug: boolean,
): CategoryTab[] {
  const main = ["image", "pdf", "web", "misc"]
  const keys = debug ? [...main, "dev"] : main
  return keys.map((key) => ({
    key,
    label: CATEGORY_LABELS[key] ?? key,
    disabled: !tools.some((t) => t.category === key),
  }))
}
