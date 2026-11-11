import { CATEGORY_LABELS } from "@/entities/tool"

export type CategoryTab = {
  key: string
  label: string
}

/** 주 탭(이미지·PDF·영상·웹폰트·기타)은 항상 보이고 누를 수 있다(도구가 없으면 준비 중 화면). 개발용은 개발 빌드에서만 맨 끝 */
export function categoryTabs(debug: boolean): CategoryTab[] {
  const main = ["image", "pdf", "video", "web", "misc"]
  const keys = debug ? [...main, "dev"] : main
  return keys.map((key) => ({ key, label: CATEGORY_LABELS[key] ?? key }))
}
