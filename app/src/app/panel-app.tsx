import { PanelWindow } from "@/widgets/tool-workspace"

import { Providers } from "./providers"
import { ThemeSync } from "./theme-sync"

/** 옵션 패널 별도 창(라벨 `panel`)의 앱: 메인과 같은 공급자·테마 */
export function PanelApp() {
  return (
    <Providers>
      <ThemeSync />
      <PanelWindow />
    </Providers>
  )
}
