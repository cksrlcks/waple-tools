import type { Settings } from "../model/types"

export const THEME_OPTIONS: { value: Settings["theme"]; label: string }[] = [
  { value: "system", label: "시스템" },
  { value: "light", label: "라이트" },
  { value: "dark", label: "다크" },
]
