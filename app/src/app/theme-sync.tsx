import { useEffect } from "react"

import { applyTheme, useSettings } from "@/entities/settings"

/** 설정의 테마를 `<html>`에 반영한다 (그리는 것 없음) */
export function ThemeSync() {
  const theme = useSettings().data?.theme ?? "system"
  useEffect(() => applyTheme(theme), [theme])
  return null
}
