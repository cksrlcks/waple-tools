export const CATEGORY_ORDER = [
  "image",
  "pdf",
  "video",
  "web",
  "misc",
  "dev",
] as const

export const CATEGORY_LABELS: Record<string, string> = {
  image: "이미지",
  pdf: "PDF",
  video: "영상",
  web: "웹폰트",
  misc: "기타",
  dev: "개발용",
}
