import { PanelWindow } from "@/widgets/tool-workspace"

import { Providers } from "./providers"

/** 옵션 패널 별도 창(라벨 `panel`)의 앱: 메인과 같은 공급자. 테마는 메인이 보내는 상태로 (`PanelWindow`) */
export function PanelApp() {
  return (
    <Providers>
      <PanelWindow />
    </Providers>
  )
}
