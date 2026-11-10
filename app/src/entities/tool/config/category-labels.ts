export const CATEGORY_ORDER = ["image", "pdf", "web", "misc", "dev"] as const

export const CATEGORY_LABELS: Record<string, string> = {
  image: "이미지",
  pdf: "PDF",
  web: "웹·폰트",
  misc: "기타",
  dev: "개발용",
}
